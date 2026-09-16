import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import {
  Keypair,
  LAMPORTS_PER_SOL,
  PublicKey,
  SystemProgram,
  Transaction,
} from "@solana/web3.js";
import {
  ASSOCIATED_TOKEN_PROGRAM_ID,
  TOKEN_PROGRAM_ID,
  createMint,
  getAccount,
  getAssociatedTokenAddressSync,
  getMint,
  getOrCreateAssociatedTokenAccount,
  mintTo,
} from "@solana/spl-token";
import { assert } from "chai";
import { Amm } from "../target/types/amm";
import ammIdl from "../target/idl/amm.json";

describe("amm", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = new Program<Amm>(ammIdl as unknown as Amm, provider);
  const connection = provider.connection;

  const authority = Keypair.generate();
  const trader = Keypair.generate();

  const DECIMALS = 6;
  const UNIT = 10 ** DECIMALS;
  const SUPPLY = 1_000_000 * UNIT;

  const SEED = new anchor.BN(Math.floor(Math.random() * 1_000_000_000));
  const FEE = 100;
  const PROTOCOL_FEE = 50;

  let mintX: PublicKey;
  let mintY: PublicKey;
  let config: PublicKey;
  let mintLp: PublicKey;
  let vaultX: PublicKey;
  let vaultY: PublicKey;
  let treasuryX: PublicKey;
  let treasuryY: PublicKey;
  let authorityX: PublicKey;
  let authorityY: PublicKey;
  let authorityLp: PublicKey;
  let traderX: PublicKey;
  let traderY: PublicKey;

  const fund = async (recipient: PublicKey, lamports: number) => {
    const tx = new Transaction().add(
      SystemProgram.transfer({
        fromPubkey: provider.wallet.publicKey,
        toPubkey: recipient,
        lamports,
      })
    );

    await provider.sendAndConfirm(tx);
  };

  const balance = async (ata: PublicKey) =>
    Number((await getAccount(connection, ata)).amount);

  const lpSupply = async () => Number((await getMint(connection, mintLp)).supply);

  const poolAccounts = (user: PublicKey, userX: PublicKey, userY: PublicKey) => ({
    user,
    mintX,
    mintY,
    config,
    mintLp,
    vaultX,
    vaultY,
    userX,
    userY,
    associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
    tokenProgram: TOKEN_PROGRAM_ID,
    systemProgram: SystemProgram.programId,
  });

  before(async () => {
    await fund(authority.publicKey, 20 * LAMPORTS_PER_SOL);
    await fund(trader.publicKey, 20 * LAMPORTS_PER_SOL);

    mintX = await createMint(
      connection,
      authority,
      authority.publicKey,
      null,
      DECIMALS
    );
    mintY = await createMint(
      connection,
      authority,
      authority.publicKey,
      null,
      DECIMALS
    );

    if (mintX.toBuffer().compare(mintY.toBuffer()) > 0) {
      [mintX, mintY] = [mintY, mintX];
    }

    config = PublicKey.findProgramAddressSync(
      [Buffer.from("config"), SEED.toArrayLike(Buffer, "le", 8)],
      program.programId
    )[0];

    mintLp = PublicKey.findProgramAddressSync(
      [Buffer.from("lp"), config.toBuffer()],
      program.programId
    )[0];

    treasuryX = PublicKey.findProgramAddressSync(
      [Buffer.from("treasury_x"), config.toBuffer()],
      program.programId
    )[0];

    treasuryY = PublicKey.findProgramAddressSync(
      [Buffer.from("treasury_y"), config.toBuffer()],
      program.programId
    )[0];

    vaultX = getAssociatedTokenAddressSync(mintX, config, true);
    vaultY = getAssociatedTokenAddressSync(mintY, config, true);

    authorityX = (
      await getOrCreateAssociatedTokenAccount(
        connection,
        authority,
        mintX,
        authority.publicKey
      )
    ).address;
    authorityY = (
      await getOrCreateAssociatedTokenAccount(
        connection,
        authority,
        mintY,
        authority.publicKey
      )
    ).address;
    traderX = (
      await getOrCreateAssociatedTokenAccount(
        connection,
        trader,
        mintX,
        trader.publicKey
      )
    ).address;
    traderY = (
      await getOrCreateAssociatedTokenAccount(
        connection,
        trader,
        mintY,
        trader.publicKey
      )
    ).address;

    authorityLp = getAssociatedTokenAddressSync(mintLp, authority.publicKey);

    await mintTo(connection, authority, mintX, authorityX, authority, SUPPLY);
    await mintTo(connection, authority, mintY, authorityY, authority, SUPPLY);
    await mintTo(connection, authority, mintX, traderX, authority, SUPPLY);
    await mintTo(connection, authority, mintY, traderY, authority, SUPPLY);
  });

  it("initializes the pool with its fee and treasury accounts", async () => {
    await program.methods
      .initialize(SEED, FEE, PROTOCOL_FEE)
      .accountsStrict({
        authority: authority.publicKey,
        mintX,
        mintY,
        config,
        mintLp,
        vaultX,
        vaultY,
        treasuryX,
        treasuryY,
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .signers([authority])
      .rpc();

    const state = await program.account.config.fetch(config);

    assert.strictEqual(state.seed.toNumber(), SEED.toNumber());
    assert.strictEqual(state.authority.toBase58(), authority.publicKey.toBase58());
    assert.strictEqual(state.mintX.toBase58(), mintX.toBase58());
    assert.strictEqual(state.mintY.toBase58(), mintY.toBase58());
    assert.strictEqual(state.fee, FEE);
    assert.strictEqual(state.protocolFee, PROTOCOL_FEE);
    assert.isFalse(state.locked);
    assert.strictEqual(await balance(treasuryX), 0);
    assert.strictEqual(await balance(treasuryY), 0);
    assert.strictEqual(await lpSupply(), 0);
  });

  it("rejects a pool whose fee is out of range", async () => {
    const seed = SEED.addn(1);
    const badConfig = PublicKey.findProgramAddressSync(
      [Buffer.from("config"), seed.toArrayLike(Buffer, "le", 8)],
      program.programId
    )[0];

    try {
      await program.methods
        .initialize(seed, 10_000, PROTOCOL_FEE)
        .accountsStrict({
          authority: authority.publicKey,
          mintX,
          mintY,
          config: badConfig,
          mintLp: PublicKey.findProgramAddressSync(
            [Buffer.from("lp"), badConfig.toBuffer()],
            program.programId
          )[0],
          vaultX: getAssociatedTokenAddressSync(mintX, badConfig, true),
          vaultY: getAssociatedTokenAddressSync(mintY, badConfig, true),
          treasuryX: PublicKey.findProgramAddressSync(
            [Buffer.from("treasury_x"), badConfig.toBuffer()],
            program.programId
          )[0],
          treasuryY: PublicKey.findProgramAddressSync(
            [Buffer.from("treasury_y"), badConfig.toBuffer()],
            program.programId
          )[0],
          associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
          tokenProgram: TOKEN_PROGRAM_ID,
          systemProgram: SystemProgram.programId,
        })
        .signers([authority])
        .rpc();
      assert.fail("a fee of 10000 basis points should have been rejected");
    } catch (err) {
      assert.strictEqual(
        (err as anchor.AnchorError).error.errorCode.code,
        "InvalidFee"
      );
    }
  });

  it("seeds the empty pool with the first deposit", async () => {
    const depositX = 100 * UNIT;
    const depositY = 400 * UNIT;
    const lp = 400 * UNIT;

    const beforeX = await balance(authorityX);
    const beforeY = await balance(authorityY);

    await program.methods
      .deposit(
        new anchor.BN(lp),
        new anchor.BN(depositX),
        new anchor.BN(depositY)
      )
      .accountsStrict({
        ...poolAccounts(authority.publicKey, authorityX, authorityY),
        userLp: authorityLp,
      })
      .signers([authority])
      .rpc();

    assert.strictEqual(await balance(vaultX), depositX);
    assert.strictEqual(await balance(vaultY), depositY);
    assert.strictEqual(await balance(authorityX), beforeX - depositX);
    assert.strictEqual(await balance(authorityY), beforeY - depositY);
    assert.strictEqual(await balance(authorityLp), lp);
    assert.strictEqual(await lpSupply(), lp);
  });

  it("adds proportional liquidity on a later deposit", async () => {
    const lp = 100 * UNIT;
    const ratioBefore = (await balance(vaultY)) / (await balance(vaultX));

    const vaultXBefore = await balance(vaultX);
    const vaultYBefore = await balance(vaultY);
    const lpBefore = await balance(authorityLp);

    await program.methods
      .deposit(
        new anchor.BN(lp),
        new anchor.BN(50 * UNIT),
        new anchor.BN(200 * UNIT)
      )
      .accountsStrict({
        ...poolAccounts(authority.publicKey, authorityX, authorityY),
        userLp: authorityLp,
      })
      .signers([authority])
      .rpc();

    const ratioAfter = (await balance(vaultY)) / (await balance(vaultX));

    assert.isAbove(await balance(vaultX), vaultXBefore);
    assert.isAbove(await balance(vaultY), vaultYBefore);
    assert.strictEqual(await balance(authorityLp), lpBefore + lp);
    assert.approximately(ratioAfter, ratioBefore, 0.01);
  });

  it("rejects a deposit that would exceed the maximum amounts", async () => {
    try {
      await program.methods
        .deposit(new anchor.BN(100 * UNIT), new anchor.BN(1), new anchor.BN(1))
        .accountsStrict({
          ...poolAccounts(authority.publicKey, authorityX, authorityY),
          userLp: authorityLp,
        })
        .signers([authority])
        .rpc();
      assert.fail("deposit past the slippage limit should have been rejected");
    } catch (err) {
      assert.strictEqual(
        (err as anchor.AnchorError).error.errorCode.code,
        "SlippageExceeded"
      );
    }
  });

  it("swaps x for y, pays the treasury, and keeps the invariant growing", async () => {
    const amount = 10 * UNIT;
    const expectedProtocol = Math.floor((amount * PROTOCOL_FEE) / 10_000);

    const vaultXBefore = await balance(vaultX);
    const vaultYBefore = await balance(vaultY);
    const traderXBefore = await balance(traderX);
    const traderYBefore = await balance(traderY);
    const kBefore = vaultXBefore * vaultYBefore;

    await program.methods
      .swap(true, new anchor.BN(amount), new anchor.BN(1))
      .accountsStrict({
        ...poolAccounts(trader.publicKey, traderX, traderY),
        treasuryX,
        treasuryY,
      })
      .signers([trader])
      .rpc();

    const received = (await balance(traderY)) - traderYBefore;

    assert.strictEqual(await balance(treasuryX), expectedProtocol);
    assert.strictEqual(await balance(traderX), traderXBefore - amount);
    assert.isAbove(received, 0);
    assert.strictEqual(await balance(vaultY), vaultYBefore - received);
    assert.isAbove(await balance(vaultX), vaultXBefore);
    assert.isAtLeast((await balance(vaultX)) * (await balance(vaultY)), kBefore);
  });

  it("swaps y for x and routes the protocol fee to the y treasury", async () => {
    const amount = 20 * UNIT;
    const expectedProtocol = Math.floor((amount * PROTOCOL_FEE) / 10_000);

    const vaultXBefore = await balance(vaultX);
    const traderXBefore = await balance(traderX);

    await program.methods
      .swap(false, new anchor.BN(amount), new anchor.BN(1))
      .accountsStrict({
        ...poolAccounts(trader.publicKey, traderX, traderY),
        treasuryX,
        treasuryY,
      })
      .signers([trader])
      .rpc();

    assert.strictEqual(await balance(treasuryY), expectedProtocol);
    assert.isAbove(await balance(traderX), traderXBefore);
    assert.isBelow(await balance(vaultX), vaultXBefore);
  });

  it("rejects a swap whose output misses the minimum", async () => {
    try {
      await program.methods
        .swap(true, new anchor.BN(UNIT), new anchor.BN(1_000_000 * UNIT))
        .accountsStrict({
          ...poolAccounts(trader.publicKey, traderX, traderY),
          treasuryX,
          treasuryY,
        })
        .signers([trader])
        .rpc();
      assert.fail("swap past the slippage limit should have been rejected");
    } catch (err) {
      assert.strictEqual(
        (err as anchor.AnchorError).error.errorCode.code,
        "SlippageExceeded"
      );
    }
  });

  it("lets the authority claim the accumulated treasury fees", async () => {
    const treasuryXBefore = await balance(treasuryX);
    const treasuryYBefore = await balance(treasuryY);
    const authorityXBefore = await balance(authorityX);
    const authorityYBefore = await balance(authorityY);

    assert.isAbove(treasuryXBefore, 0);
    assert.isAbove(treasuryYBefore, 0);

    await program.methods
      .claim()
      .accountsStrict({
        authority: authority.publicKey,
        mintX,
        mintY,
        config,
        treasuryX,
        treasuryY,
        authorityX,
        authorityY,
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .signers([authority])
      .rpc();

    assert.strictEqual(await balance(treasuryX), 0);
    assert.strictEqual(await balance(treasuryY), 0);
    assert.strictEqual(
      await balance(authorityX),
      authorityXBefore + treasuryXBefore
    );
    assert.strictEqual(
      await balance(authorityY),
      authorityYBefore + treasuryYBefore
    );
  });

  it("refuses a claim when the treasury is empty", async () => {
    try {
      await program.methods
        .claim()
        .accountsStrict({
          authority: authority.publicKey,
          mintX,
          mintY,
          config,
          treasuryX,
          treasuryY,
          authorityX,
          authorityY,
          associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
          tokenProgram: TOKEN_PROGRAM_ID,
          systemProgram: SystemProgram.programId,
        })
        .signers([authority])
        .rpc();
      assert.fail("claiming an empty treasury should have been rejected");
    } catch (err) {
      assert.strictEqual(
        (err as anchor.AnchorError).error.errorCode.code,
        "NothingToClaim"
      );
    }
  });

  it("refuses a claim from anyone but the pool authority", async () => {
    try {
      await program.methods
        .claim()
        .accountsStrict({
          authority: trader.publicKey,
          mintX,
          mintY,
          config,
          treasuryX,
          treasuryY,
          authorityX: traderX,
          authorityY: traderY,
          associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
          tokenProgram: TOKEN_PROGRAM_ID,
          systemProgram: SystemProgram.programId,
        })
        .signers([trader])
        .rpc();
      assert.fail("a non authority claim should have been rejected");
    } catch (err) {
      assert.strictEqual(
        (err as anchor.AnchorError).error.errorCode.code,
        "ConstraintHasOne"
      );
    }
  });

  it("burns lp tokens and returns the underlying on withdraw", async () => {
    const lp = 50 * UNIT;

    const vaultXBefore = await balance(vaultX);
    const vaultYBefore = await balance(vaultY);
    const userXBefore = await balance(authorityX);
    const userYBefore = await balance(authorityY);
    const lpBefore = await balance(authorityLp);
    const supplyBefore = await lpSupply();

    await program.methods
      .withdraw(new anchor.BN(lp), new anchor.BN(1), new anchor.BN(1))
      .accountsStrict({
        ...poolAccounts(authority.publicKey, authorityX, authorityY),
        userLp: authorityLp,
      })
      .signers([authority])
      .rpc();

    assert.strictEqual(await balance(authorityLp), lpBefore - lp);
    assert.strictEqual(await lpSupply(), supplyBefore - lp);
    assert.isBelow(await balance(vaultX), vaultXBefore);
    assert.isBelow(await balance(vaultY), vaultYBefore);
    assert.isAbove(await balance(authorityX), userXBefore);
    assert.isAbove(await balance(authorityY), userYBefore);
  });

  it("rejects a withdrawal that would return less than the minimum", async () => {
    try {
      await program.methods
        .withdraw(
          new anchor.BN(UNIT),
          new anchor.BN(1_000_000 * UNIT),
          new anchor.BN(1)
        )
        .accountsStrict({
          ...poolAccounts(authority.publicKey, authorityX, authorityY),
          userLp: authorityLp,
        })
        .signers([authority])
        .rpc();
      assert.fail("withdrawal past the slippage limit should have been rejected");
    } catch (err) {
      assert.strictEqual(
        (err as anchor.AnchorError).error.errorCode.code,
        "SlippageExceeded"
      );
    }
  });

  it("locks the pool and refuses swaps until it is unlocked", async () => {
    await program.methods
      .setLocked(true)
      .accountsStrict({ authority: authority.publicKey, config })
      .signers([authority])
      .rpc();

    assert.isTrue((await program.account.config.fetch(config)).locked);

    try {
      await program.methods
        .swap(true, new anchor.BN(UNIT), new anchor.BN(1))
        .accountsStrict({
          ...poolAccounts(trader.publicKey, traderX, traderY),
          treasuryX,
          treasuryY,
        })
        .signers([trader])
        .rpc();
      assert.fail("a swap on a locked pool should have been rejected");
    } catch (err) {
      assert.strictEqual(
        (err as anchor.AnchorError).error.errorCode.code,
        "PoolLocked"
      );
    }

    await program.methods
      .setLocked(false)
      .accountsStrict({ authority: authority.publicKey, config })
      .signers([authority])
      .rpc();

    assert.isFalse((await program.account.config.fetch(config)).locked);
  });
});
