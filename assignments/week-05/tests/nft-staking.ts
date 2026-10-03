import * as anchor from "@coral-xyz/anchor";
import { AnchorError, BN, Program } from "@coral-xyz/anchor";
import {
  Connection,
  Keypair,
  LAMPORTS_PER_SOL,
  PublicKey,
  SystemProgram,
  Transaction,
} from "@solana/web3.js";
import {
  ASSOCIATED_TOKEN_PROGRAM_ID,
  TOKEN_PROGRAM_ID,
  getAccount,
  getAssociatedTokenAddressSync,
  getMint,
} from "@solana/spl-token";
import { createUmi } from "@metaplex-foundation/umi-bundle-defaults";
import {
  keypairIdentity,
  publicKey as umiPublicKey,
} from "@metaplex-foundation/umi";
import {
  fetchAsset,
  fetchCollection,
  mplCore,
  transferV1,
} from "@metaplex-foundation/mpl-core";
import { assert } from "chai";
import { NftStaking } from "../target/types/nft_staking";
import idl from "../target/idl/nft_staking.json";

const CORE_PROGRAM_ID = new PublicKey(
  "CoREENxT6tW1HoK8ypY1SxRMZTcVPm7R94rH4PZNhX7d"
);
const UNIT = 1_000_000;
const RATE = 10 * UNIT;
const BONUS = 1_000 * UNIT;
const URI = "https://example.com/staking.json";

type Pool = {
  collection: Keypair;
  updateAuthority: PublicKey;
  config: PublicKey;
  rewardsMint: PublicKey;
};

describe("nft-staking", () => {
  const env = anchor.AnchorProvider.env();
  const provider = new anchor.AnchorProvider(
    new Connection(env.connection.rpcEndpoint, "confirmed"),
    env.wallet,
    { commitment: "confirmed", preflightCommitment: "confirmed" }
  );
  anchor.setProvider(provider);

  const program = new Program<NftStaking>(
    idl as unknown as NftStaking,
    provider
  );
  const connection = provider.connection;
  const admin = provider.wallet.publicKey;
  const umi = createUmi(connection.rpcEndpoint, "confirmed").use(mplCore());

  const alice = Keypair.generate();
  const mallory = Keypair.generate();

  const pda = (seeds: Buffer[]) =>
    PublicKey.findProgramAddressSync(seeds, program.programId)[0];

  const poolFor = (collection: Keypair): Pool => {
    const config = pda([Buffer.from("config"), collection.publicKey.toBuffer()]);

    return {
      collection,
      updateAuthority: pda([
        Buffer.from("update_authority"),
        collection.publicKey.toBuffer(),
      ]),
      config,
      rewardsMint: pda([Buffer.from("rewards"), config.toBuffer()]),
    };
  };

  const fast = poolFor(Keypair.generate());
  const slow = poolFor(Keypair.generate());

  let fastOne: PublicKey;
  let fastTwo: PublicKey;
  let slowOne: PublicKey;
  let slowTwo: PublicKey;

  const sleep = (ms: number) => new Promise((resolve) => setTimeout(resolve, ms));

  const chainNow = async () =>
    (await connection.getBlockTime(await connection.getSlot())) as number;

  const fund = async (recipient: PublicKey, sol: number) => {
    await provider.sendAndConfirm(
      new Transaction().add(
        SystemProgram.transfer({
          fromPubkey: admin,
          toPubkey: recipient,
          lamports: sol * LAMPORTS_PER_SOL,
        })
      )
    );
  };

  const initialize = (pool: Pool, periodSeconds: number, minStakeSeconds: number) =>
    program.methods
      .initialize({
        name: "Staking Collection",
        uri: URI,
        rewardsPerPeriod: new BN(RATE),
        periodSeconds: new BN(periodSeconds),
        minStakeSeconds: new BN(minStakeSeconds),
        burnBonus: new BN(BONUS),
      })
      .accountsStrict({
        admin,
        collection: pool.collection.publicKey,
        updateAuthority: pool.updateAuthority,
        config: pool.config,
        rewardsMint: pool.rewardsMint,
        coreProgram: CORE_PROGRAM_ID,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .signers([pool.collection])
      .rpc();

  const mintAsset = async (pool: Pool, owner: PublicKey, name: string) => {
    const asset = Keypair.generate();

    await program.methods
      .mintAsset(name, URI)
      .accountsStrict({
        admin,
        owner,
        asset: asset.publicKey,
        collection: pool.collection.publicKey,
        updateAuthority: pool.updateAuthority,
        config: pool.config,
        coreProgram: CORE_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .signers([asset])
      .rpc();

    return asset.publicKey;
  };

  const stakeAccounts = (pool: Pool, asset: PublicKey, owner: PublicKey) => ({
    owner,
    asset,
    collection: pool.collection.publicKey,
    updateAuthority: pool.updateAuthority,
    config: pool.config,
    coreProgram: CORE_PROGRAM_ID,
    systemProgram: SystemProgram.programId,
  });

  const positionAccounts = (pool: Pool, asset: PublicKey, owner: PublicKey) => ({
    ...stakeAccounts(pool, asset, owner),
    rewardsMint: pool.rewardsMint,
    ownerRewards: getAssociatedTokenAddressSync(pool.rewardsMint, owner),
    tokenProgram: TOKEN_PROGRAM_ID,
    associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
  });

  const stake = (pool: Pool, asset: PublicKey, owner: Keypair) =>
    program.methods
      .stake()
      .accountsStrict(stakeAccounts(pool, asset, owner.publicKey))
      .signers([owner])
      .rpc();

  const claim = (pool: Pool, asset: PublicKey, owner: Keypair) =>
    program.methods
      .claimRewards()
      .accountsStrict(positionAccounts(pool, asset, owner.publicKey))
      .signers([owner])
      .rpc();

  const unstake = (pool: Pool, asset: PublicKey, owner: Keypair) =>
    program.methods
      .unstake()
      .accountsStrict(positionAccounts(pool, asset, owner.publicKey))
      .signers([owner])
      .rpc();

  const burn = (pool: Pool, asset: PublicKey, owner: Keypair) =>
    program.methods
      .burnStakedNft()
      .accountsStrict(positionAccounts(pool, asset, owner.publicKey))
      .signers([owner])
      .rpc();

  const loadAsset = (asset: PublicKey) =>
    fetchAsset(umi, umiPublicKey(asset.toBase58()));

  const attributesOf = async (asset: PublicKey) =>
    Object.fromEntries(
      ((await loadAsset(asset)).attributes?.attributeList ?? []).map(
        ({ key, value }) => [key, value]
      )
    );

  const totalStaked = async (pool: Pool) => {
    const collection = await fetchCollection(
      umi,
      umiPublicKey(pool.collection.publicKey.toBase58())
    );

    return Number(
      collection.attributes?.attributeList.find(
        ({ key }) => key === "total_staked"
      )?.value
    );
  };

  const rewardsOf = async (pool: Pool, owner: PublicKey) => {
    const account = getAssociatedTokenAddressSync(pool.rewardsMint, owner);

    if (!(await connection.getAccountInfo(account))) {
      return 0;
    }

    return Number((await getAccount(connection, account)).amount);
  };

  const transferAsset = (pool: Pool, asset: PublicKey, owner: Keypair, to: PublicKey) => {
    const ownerUmi = createUmi(connection.rpcEndpoint, "confirmed")
      .use(mplCore())
      .use(keypairIdentity(umi.eddsa.createKeypairFromSecretKey(owner.secretKey)));

    return transferV1(ownerUmi, {
      asset: umiPublicKey(asset.toBase58()),
      collection: umiPublicKey(pool.collection.publicKey.toBase58()),
      newOwner: umiPublicKey(to.toBase58()),
    }).sendAndConfirm(ownerUmi);
  };

  const expectError = async (action: Promise<unknown>, code: string) => {
    let caught: unknown;

    try {
      await action;
    } catch (err) {
      caught = err;
    }

    assert.exists(caught, `expected ${code}`);
    assert.strictEqual((caught as AnchorError).error?.errorCode?.code, code);
  };

  before(async () => {
    await fund(alice.publicKey, 10);
    await fund(mallory.publicKey, 10);
  });

  describe("initialize", () => {
    it("creates the collection under a program owned update authority", async () => {
      await initialize(fast, 1, 3);
      await initialize(slow, 3600, 3600);

      const collection = await fetchCollection(
        umi,
        umiPublicKey(fast.collection.publicKey.toBase58())
      );
      const config = await program.account.config.fetch(fast.config);
      const mint = await getMint(connection, fast.rewardsMint);

      assert.strictEqual(collection.updateAuthority, fast.updateAuthority.toBase58());
      assert.strictEqual(await totalStaked(fast), 0);
      assert.strictEqual(config.admin.toBase58(), admin.toBase58());
      assert.strictEqual(config.rewardsPerPeriod.toNumber(), RATE);
      assert.strictEqual(config.periodSeconds.toNumber(), 1);
      assert.strictEqual(config.minStakeSeconds.toNumber(), 3);
      assert.strictEqual(config.burnBonus.toNumber(), BONUS);
      assert.strictEqual(mint.mintAuthority?.toBase58(), fast.config.toBase58());
      assert.strictEqual(mint.supply, BigInt(0));
    });

    it("rejects a pool with a zero reward period", async () => {
      await expectError(initialize(poolFor(Keypair.generate()), 0, 0), "InvalidPeriod");
    });
  });

  describe("mint_asset", () => {
    it("mints assets into the collection for a holder", async () => {
      fastOne = await mintAsset(fast, alice.publicKey, "Fast #1");
      fastTwo = await mintAsset(fast, alice.publicKey, "Fast #2");
      slowOne = await mintAsset(slow, alice.publicKey, "Slow #1");
      slowTwo = await mintAsset(slow, alice.publicKey, "Slow #2");

      const asset = await loadAsset(fastOne);
      const collection = await fetchCollection(
        umi,
        umiPublicKey(fast.collection.publicKey.toBase58())
      );

      assert.strictEqual(asset.owner, alice.publicKey.toBase58());
      assert.strictEqual(asset.updateAuthority.type, "Collection");
      assert.strictEqual(
        asset.updateAuthority.address,
        fast.collection.publicKey.toBase58()
      );
      assert.strictEqual(collection.currentSize, 2);
    });

    it("rejects a mint from anyone but the pool admin", async () => {
      const asset = Keypair.generate();

      await expectError(
        program.methods
          .mintAsset("Rogue", URI)
          .accountsStrict({
            admin: mallory.publicKey,
            owner: mallory.publicKey,
            asset: asset.publicKey,
            collection: fast.collection.publicKey,
            updateAuthority: fast.updateAuthority,
            config: fast.config,
            coreProgram: CORE_PROGRAM_ID,
            systemProgram: SystemProgram.programId,
          })
          .signers([mallory, asset])
          .rpc(),
        "ConstraintHasOne"
      );
    });
  });

  describe("stake", () => {
    it("freezes the asset, delegates burning and counts it on the collection", async () => {
      await stake(fast, fastOne, alice);

      const asset = await loadAsset(fastOne);
      const attributes = await attributesOf(fastOne);

      assert.isTrue(asset.freezeDelegate?.frozen);
      assert.strictEqual(asset.freezeDelegate?.authority.type, "Address");
      assert.strictEqual(
        asset.freezeDelegate?.authority.address,
        fast.updateAuthority.toBase58()
      );
      assert.strictEqual(asset.burnDelegate?.authority.type, "Address");
      assert.strictEqual(
        asset.burnDelegate?.authority.address,
        fast.updateAuthority.toBase58()
      );
      assert.strictEqual(attributes.staked, "true");
      assert.strictEqual(attributes.staked_at, attributes.last_claimed);
      assert.strictEqual(await totalStaked(fast), 1);
    });

    it("keeps a staked asset from being transferred by its owner", async () => {
      let caught: unknown;

      try {
        await transferAsset(fast, fastOne, alice, mallory.publicKey);
      } catch (err) {
        caught = err;
      }

      assert.exists(caught);
      assert.strictEqual((await loadAsset(fastOne)).owner, alice.publicKey.toBase58());
    });

    it("rejects staking an asset that is already staked", async () => {
      await expectError(stake(fast, fastOne, alice), "AlreadyStaked");
    });

    it("rejects staking by someone who does not own the asset", async () => {
      await expectError(stake(fast, fastTwo, mallory), "NotOwner");
    });

    it("tracks every staked asset in total_staked", async () => {
      await stake(fast, fastTwo, alice);

      assert.strictEqual(await totalStaked(fast), 2);
    });
  });

  describe("claim_rewards", () => {
    it("refuses a claim before a full reward period has passed", async () => {
      await stake(slow, slowOne, alice);

      assert.strictEqual(await totalStaked(slow), 1);
      await expectError(claim(slow, slowOne, alice), "NothingToClaim");
    });

    it("mints accrued rewards while the asset stays staked and frozen", async () => {
      await sleep(2500);

      const before = await attributesOf(fastOne);
      const balanceBefore = await rewardsOf(fast, alice.publicKey);

      await claim(fast, fastOne, alice);

      const after = await attributesOf(fastOne);
      const asset = await loadAsset(fastOne);
      const periods = Number(after.last_claimed) - Number(before.last_claimed);

      assert.isAtLeast(periods, 2);
      assert.strictEqual(
        (await rewardsOf(fast, alice.publicKey)) - balanceBefore,
        periods * RATE
      );
      assert.isTrue(asset.freezeDelegate?.frozen);
      assert.strictEqual(after.staked, "true");
      assert.strictEqual(after.staked_at, before.staked_at);
      assert.strictEqual(await totalStaked(fast), 2);
    });

    it("refuses a claim on an asset that is not staked", async () => {
      await expectError(claim(slow, slowTwo, alice), "NotStaked");
    });
  });

  describe("unstake", () => {
    it("refuses to unstake inside the minimum staking period", async () => {
      await expectError(unstake(slow, slowOne, alice), "StakeLocked");
    });

    it("pays out, thaws and returns the asset once the period has passed", async () => {
      const before = await attributesOf(fastOne);

      while ((await chainNow()) < Number(before.staked_at) + 4) {
        await sleep(500);
      }

      const balanceBefore = await rewardsOf(fast, alice.publicKey);

      await unstake(fast, fastOne, alice);

      const after = await attributesOf(fastOne);
      const asset = await loadAsset(fastOne);
      const periods = Number(after.last_claimed) - Number(before.last_claimed);

      assert.strictEqual(
        (await rewardsOf(fast, alice.publicKey)) - balanceBefore,
        periods * RATE
      );
      assert.isUndefined(asset.freezeDelegate);
      assert.isUndefined(asset.burnDelegate);
      assert.strictEqual(after.staked, "false");
      assert.strictEqual(await totalStaked(fast), 1);
    });

    it("lets the owner transfer the asset after unstaking", async () => {
      await transferAsset(fast, fastOne, alice, mallory.publicKey);

      assert.strictEqual((await loadAsset(fastOne)).owner, mallory.publicKey.toBase58());
    });
  });

  describe("burn_staked_nft", () => {
    it("burns a staked asset for the one time bonus", async () => {
      const balanceBefore = await rewardsOf(slow, alice.publicKey);
      const sizeBefore = (
        await fetchCollection(umi, umiPublicKey(slow.collection.publicKey.toBase58()))
      ).currentSize;

      await burn(slow, slowOne, alice);

      const collection = await fetchCollection(
        umi,
        umiPublicKey(slow.collection.publicKey.toBase58())
      );
      const account = await connection.getAccountInfo(slowOne);

      let decoded = true;
      try {
        await loadAsset(slowOne);
      } catch {
        decoded = false;
      }

      assert.strictEqual((await rewardsOf(slow, alice.publicKey)) - balanceBefore, BONUS);
      assert.deepEqual([...(account?.data ?? [])], [0]);
      assert.isFalse(decoded);
      assert.strictEqual(collection.currentSize, sizeBefore - 1);
      assert.strictEqual(await totalStaked(slow), 0);
    });

    it("refuses to burn an asset that is not staked", async () => {
      await expectError(burn(slow, slowTwo, alice), "NotStaked");
    });
  });
});
