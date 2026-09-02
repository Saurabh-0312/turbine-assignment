import { address } from "@solana/kit";
import { fetchMint, fetchToken } from "@solana-program/token";
import { fetchMetadataFromSeeds } from "@metaplex-foundation/mpl-token-metadata";
import { publicKey } from "@metaplex-foundation/umi";
import { beforeAll, describe, expect, it } from "vitest";
import { readArtifacts, type Artifacts } from "../src/lib/artifacts";
import { MINT_AMOUNT, TOKEN_DECIMALS, TOKEN_NAME, TOKEN_SYMBOL, TOKEN_URI, TRANSFER_AMOUNT } from "../src/lib/config";
import { createUmiClient, rpc } from "../src/lib/rpc";
import { loadSigner } from "../src/lib/wallet";

describe("spl token", () => {
  let artifacts: Artifacts;
  let owner: string;

  beforeAll(async () => {
    artifacts = readArtifacts();
    owner = (await loadSigner()).address;
  });

  it("recorded a mint address", () => {
    expect(artifacts.mint).toBeTypeOf("string");
    expect(artifacts.mint!.length).toBeGreaterThan(31);
  });

  it("created the mint with the configured decimals", async () => {
    const mint = await fetchMint(rpc, address(artifacts.mint!));
    expect(mint.data.decimals).toBe(TOKEN_DECIMALS);
  });

  it("kept the wallet as mint authority", async () => {
    const mint = await fetchMint(rpc, address(artifacts.mint!));
    const authority = mint.data.mintAuthority;
    expect(authority.__option).toBe("Some");
    if (authority.__option !== "Some") throw new Error("mint authority was revoked");
    expect(String(authority.value)).toBe(owner);
  });

  it("attached on-chain metadata to the mint", async () => {
    const umi = createUmiClient();
    const metadata = await fetchMetadataFromSeeds(umi, { mint: publicKey(artifacts.mint!) });
    expect(metadata.name.replace(/\0/g, "")).toBe(TOKEN_NAME);
    expect(metadata.symbol.replace(/\0/g, "")).toBe(TOKEN_SYMBOL);
    expect(metadata.uri.replace(/\0/g, "")).toBe(TOKEN_URI);
  });

  it("minted the full supply into the owner token account", async () => {
    const account = await fetchToken(rpc, address(artifacts.ownerAta!));
    expect(String(account.data.mint)).toBe(artifacts.mint);
    expect(String(account.data.owner)).toBe(owner);
    expect(account.data.amount).toBe(MINT_AMOUNT - TRANSFER_AMOUNT);
  });

  it("transferred the configured amount to the recipient", async () => {
    const account = await fetchToken(rpc, address(artifacts.recipientAta!));
    expect(String(account.data.mint)).toBe(artifacts.mint);
    expect(String(account.data.owner)).toBe(artifacts.recipient);
    expect(account.data.amount).toBe(TRANSFER_AMOUNT);
  });

  it("conserved the total supply across both accounts", async () => {
    const [ownerAccount, recipientAccount] = await Promise.all([
      fetchToken(rpc, address(artifacts.ownerAta!)),
      fetchToken(rpc, address(artifacts.recipientAta!)),
    ]);
    expect(ownerAccount.data.amount + recipientAccount.data.amount).toBe(MINT_AMOUNT);
  });
});
