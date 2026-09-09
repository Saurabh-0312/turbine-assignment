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
  getOrCreateAssociatedTokenAccount,
  mintTo,
} from "@solana/spl-token";
import { assert } from "chai";
import { Escrow } from "../target/types/escrow";
import escrowIdl from "../target/idl/escrow.json";

describe("escrow", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = new Program<Escrow>(escrowIdl as unknown as Escrow, provider);
  const connection = provider.connection;

  const maker = Keypair.generate();
  const taker = Keypair.generate();

  const DECIMALS = 6;
  const UNIT = 10 ** DECIMALS;
  const SUPPLY = 1000 * UNIT;

  let mintA: PublicKey;
  let mintB: PublicKey;
  let makerAtaA: PublicKey;
  let takerAtaB: PublicKey;
  let takerAtaA: PublicKey;
  let makerAtaB: PublicKey;

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

  const escrowPda = (seed: anchor.BN) =>
    PublicKey.findProgramAddressSync(
      [
        Buffer.from("escrow"),
        maker.publicKey.toBuffer(),
        seed.toArrayLike(Buffer, "le", 8),
      ],
      program.programId
    )[0];

  const vaultOf = (escrow: PublicKey) =>
    getAssociatedTokenAddressSync(mintA, escrow, true);

  const chainNow = async () => {
    const slot = await connection.getSlot();
    return (await connection.getBlockTime(slot)) as number;
  };

  const amountOf = async (ata: PublicKey) =>
    Number((await getAccount(connection, ata)).amount);

  const makeEscrow = async (
    seed: anchor.BN,
    deposit: number,
    receive: number,
    expiry: number
  ) => {
    const escrow = escrowPda(seed);

    await program.methods
      .make(
        seed,
        new anchor.BN(deposit),
        new anchor.BN(receive),
        new anchor.BN(expiry)
      )
      .accountsStrict({
        maker: maker.publicKey,
        mintA,
        mintB,
        makerAtaA,
        escrow,
        vault: vaultOf(escrow),
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .signers([maker])
      .rpc();

    return escrow;
  };

  before(async () => {
    await fund(maker.publicKey, 20 * LAMPORTS_PER_SOL);
    await fund(taker.publicKey, 20 * LAMPORTS_PER_SOL);

    mintA = await createMint(
      connection,
      maker,
      maker.publicKey,
      null,
      DECIMALS
    );
    mintB = await createMint(
      connection,
      taker,
      taker.publicKey,
      null,
      DECIMALS
    );

    makerAtaA = (
      await getOrCreateAssociatedTokenAccount(
        connection,
        maker,
        mintA,
        maker.publicKey
      )
    ).address;
    takerAtaB = (
      await getOrCreateAssociatedTokenAccount(
        connection,
        taker,
        mintB,
        taker.publicKey
      )
    ).address;

    takerAtaA = getAssociatedTokenAddressSync(mintA, taker.publicKey);
    makerAtaB = getAssociatedTokenAddressSync(mintB, maker.publicKey);

    await mintTo(connection, maker, mintA, makerAtaA, maker, SUPPLY);
    await mintTo(connection, taker, mintB, takerAtaB, taker, SUPPLY);
  });

  it("makes an escrow and locks the deposit in the vault", async () => {
    const seed = new anchor.BN(1);
    const deposit = 10 * UNIT;
    const receive = 5 * UNIT;
    const expiry = (await chainNow()) + 3600;

    const makerBefore = await amountOf(makerAtaA);
    const escrow = await makeEscrow(seed, deposit, receive, expiry);
    const state = await program.account.escrow.fetch(escrow);

    assert.strictEqual(state.maker.toBase58(), maker.publicKey.toBase58());
    assert.strictEqual(state.mintA.toBase58(), mintA.toBase58());
    assert.strictEqual(state.mintB.toBase58(), mintB.toBase58());
    assert.strictEqual(state.receive.toNumber(), receive);
    assert.strictEqual(state.expiry.toNumber(), expiry);
    assert.strictEqual(await amountOf(vaultOf(escrow)), deposit);
    assert.strictEqual(await amountOf(makerAtaA), makerBefore - deposit);
  });

  it("rejects a make whose expiry is already in the past", async () => {
    try {
      await makeEscrow(new anchor.BN(99), 1 * UNIT, 1 * UNIT, 1);
      assert.fail("expired escrow should have been rejected");
    } catch (err) {
      assert.strictEqual(
        (err as anchor.AnchorError).error.errorCode.code,
        "InvalidExpiry"
      );
    }
  });

  it("rejects a make where both mints are the same", async () => {
    const seed = new anchor.BN(98);
    const escrow = escrowPda(seed);

    try {
      await program.methods
        .make(
          seed,
          new anchor.BN(UNIT),
          new anchor.BN(UNIT),
          new anchor.BN((await chainNow()) + 3600)
        )
        .accountsStrict({
          maker: maker.publicKey,
          mintA,
          mintB: mintA,
          makerAtaA,
          escrow,
          vault: vaultOf(escrow),
          associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
          tokenProgram: TOKEN_PROGRAM_ID,
          systemProgram: SystemProgram.programId,
        })
        .signers([maker])
        .rpc();
      assert.fail("identical mints should have been rejected");
    } catch (err) {
      assert.strictEqual(
        (err as anchor.AnchorError).error.errorCode.code,
        "IdenticalMints"
      );
    }
  });

  it("lets the maker update the receive amount and the expiry", async () => {
    const seed = new anchor.BN(1);
    const escrow = escrowPda(seed);
    const receive = 7 * UNIT;
    const expiry = (await chainNow()) + 7200;

    await program.methods
      .update(new anchor.BN(receive), new anchor.BN(expiry))
      .accountsStrict({ maker: maker.publicKey, escrow })
      .signers([maker])
      .rpc();

    const state = await program.account.escrow.fetch(escrow);

    assert.strictEqual(state.receive.toNumber(), receive);
    assert.strictEqual(state.expiry.toNumber(), expiry);
  });

  it("completes the swap when the taker takes the escrow", async () => {
    const seed = new anchor.BN(1);
    const escrow = escrowPda(seed);
    const state = await program.account.escrow.fetch(escrow);
    const locked = await amountOf(vaultOf(escrow));
    const takerBefore = await amountOf(takerAtaB);

    await program.methods
      .take()
      .accountsStrict({
        taker: taker.publicKey,
        maker: maker.publicKey,
        mintA,
        mintB,
        takerAtaA,
        takerAtaB,
        makerAtaB,
        escrow,
        vault: vaultOf(escrow),
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .signers([taker])
      .rpc();

    assert.strictEqual(await amountOf(takerAtaA), locked);
    assert.strictEqual(await amountOf(makerAtaB), state.receive.toNumber());
    assert.strictEqual(
      await amountOf(takerAtaB),
      takerBefore - state.receive.toNumber()
    );
    assert.isNull(await connection.getAccountInfo(escrow));
    assert.isNull(await connection.getAccountInfo(vaultOf(escrow)));
  });

  it("refunds an unclaimed escrow back to the maker", async () => {
    const seed = new anchor.BN(2);
    const deposit = 4 * UNIT;
    const expiry = (await chainNow()) + 3600;

    const escrow = await makeEscrow(seed, deposit, 2 * UNIT, expiry);
    const makerBefore = await amountOf(makerAtaA);

    await program.methods
      .refund()
      .accountsStrict({
        maker: maker.publicKey,
        mintA,
        makerAtaA,
        escrow,
        vault: vaultOf(escrow),
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .signers([maker])
      .rpc();

    assert.strictEqual(await amountOf(makerAtaA), makerBefore + deposit);
    assert.isNull(await connection.getAccountInfo(escrow));
    assert.isNull(await connection.getAccountInfo(vaultOf(escrow)));
  });

  it("refuses a take once the escrow has expired", async () => {
    const seed = new anchor.BN(3);
    const escrow = await makeEscrow(
      seed,
      3 * UNIT,
      1 * UNIT,
      (await chainNow()) + 2
    );

    await new Promise((resolve) => setTimeout(resolve, 6000));

    try {
      await program.methods
        .take()
        .accountsStrict({
          taker: taker.publicKey,
          maker: maker.publicKey,
          mintA,
          mintB,
          takerAtaA,
          takerAtaB,
          makerAtaB,
          escrow,
          vault: vaultOf(escrow),
          associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
          tokenProgram: TOKEN_PROGRAM_ID,
          systemProgram: SystemProgram.programId,
        })
        .signers([taker])
        .rpc();
      assert.fail("take after expiry should have been rejected");
    } catch (err) {
      assert.strictEqual(
        (err as anchor.AnchorError).error.errorCode.code,
        "EscrowExpired"
      );
    }

    const state = await program.account.escrow.fetch(escrow);
    assert.isBelow(state.expiry.toNumber(), await chainNow());
  });
});
