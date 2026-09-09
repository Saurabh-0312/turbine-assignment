import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import {
  Keypair,
  LAMPORTS_PER_SOL,
  PublicKey,
  SystemProgram,
  Transaction,
} from "@solana/web3.js";
import { assert } from "chai";
import { Vault } from "../target/types/vault";
import vaultIdl from "../target/idl/vault.json";

describe("vault", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = new Program<Vault>(vaultIdl as unknown as Vault, provider);
  const connection = provider.connection;
  const user = Keypair.generate();

  const [vaultState] = PublicKey.findProgramAddressSync(
    [Buffer.from("state"), user.publicKey.toBuffer()],
    program.programId
  );

  const [vault] = PublicKey.findProgramAddressSync(
    [Buffer.from("vault"), vaultState.toBuffer()],
    program.programId
  );

  const accounts = {
    user: user.publicKey,
    vault,
    vaultState,
    systemProgram: SystemProgram.programId,
  };

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

  before(async () => {
    await fund(user.publicKey, 10 * LAMPORTS_PER_SOL);
  });

  it("initializes a vault owned by the user", async () => {
    await program.methods
      .initialize()
      .accountsStrict({
        user: user.publicKey,
        vaultState,
        vault,
        systemProgram: SystemProgram.programId,
      })
      .signers([user])
      .rpc();

    const state = await program.account.vaultState.fetch(vaultState);

    assert.strictEqual(state.owner.toBase58(), user.publicKey.toBase58());
    assert.isAbove(state.vaultBump, 0);
    assert.isAbove(state.stateBump, 0);
  });

  it("deposits lamports into the vault", async () => {
    const amount = 2 * LAMPORTS_PER_SOL;
    const before = await connection.getBalance(vault);

    await program.methods
      .deposit(new anchor.BN(amount))
      .accountsStrict(accounts)
      .signers([user])
      .rpc();

    assert.strictEqual(await connection.getBalance(vault), before + amount);
  });

  it("rejects a deposit of zero", async () => {
    try {
      await program.methods
        .deposit(new anchor.BN(0))
        .accountsStrict(accounts)
        .signers([user])
        .rpc();
      assert.fail("deposit of zero should have been rejected");
    } catch (err) {
      assert.strictEqual(
        (err as anchor.AnchorError).error.errorCode.code,
        "InvalidAmount"
      );
    }
  });

  it("withdraws part of the balance back to the user", async () => {
    const amount = LAMPORTS_PER_SOL;
    const vaultBefore = await connection.getBalance(vault);
    const userBefore = await connection.getBalance(user.publicKey);

    await program.methods
      .withdraw(new anchor.BN(amount))
      .accountsStrict(accounts)
      .signers([user])
      .rpc();

    assert.strictEqual(await connection.getBalance(vault), vaultBefore - amount);
    assert.isAbove(await connection.getBalance(user.publicKey), userBefore);
  });

  it("rejects a withdrawal larger than the vault balance", async () => {
    const balance = await connection.getBalance(vault);

    try {
      await program.methods
        .withdraw(new anchor.BN(balance + LAMPORTS_PER_SOL))
        .accountsStrict(accounts)
        .signers([user])
        .rpc();
      assert.fail("overdrawn withdrawal should have been rejected");
    } catch (err) {
      assert.strictEqual(
        (err as anchor.AnchorError).error.errorCode.code,
        "InsufficientFunds"
      );
    }
  });

  it("closes the vault and returns every remaining lamport", async () => {
    const userBefore = await connection.getBalance(user.publicKey);
    const vaultBefore = await connection.getBalance(vault);

    await program.methods
      .close()
      .accountsStrict(accounts)
      .signers([user])
      .rpc();

    assert.strictEqual(await connection.getBalance(vault), 0);
    assert.isNull(await connection.getAccountInfo(vaultState));
    assert.isAbove(
      await connection.getBalance(user.publicKey),
      userBefore + vaultBefore - 10000
    );
  });
});
