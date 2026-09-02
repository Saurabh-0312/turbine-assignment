import { fetchAsset, type AssetV1 } from "@metaplex-foundation/mpl-core";
import { publicKey } from "@metaplex-foundation/umi";
import { beforeAll, describe, expect, it } from "vitest";
import { readArtifacts, type Artifacts } from "../src/lib/artifacts";
import { NFT_NAME, NFT_UPDATED_NAME, NFT_UPDATED_URI, NFT_URI } from "../src/lib/config";
import { createUmiClient } from "../src/lib/rpc";
import { loadSigner } from "../src/lib/wallet";

describe("mpl core nft", () => {
  let artifacts: Artifacts;
  let asset: AssetV1;
  let owner: string;

  beforeAll(async () => {
    artifacts = readArtifacts();
    owner = (await loadSigner()).address;
    asset = await fetchAsset(createUmiClient(), publicKey(artifacts.asset!));
  });

  it("recorded an asset address", () => {
    expect(artifacts.asset).toBeTypeOf("string");
    expect(artifacts.asset!.length).toBeGreaterThan(31);
  });

  it("minted the asset to the wallet", () => {
    expect(String(asset.owner)).toBe(owner);
  });

  it("kept the wallet as update authority", () => {
    expect(asset.updateAuthority.type).toBe("Address");
    expect(String(asset.updateAuthority.address)).toBe(owner);
  });

  it("applied the updated name", () => {
    expect(asset.name).toBe(NFT_UPDATED_NAME);
    expect(asset.name).not.toBe(NFT_NAME);
  });

  it("applied the updated metadata uri", () => {
    expect(asset.uri).toBe(NFT_UPDATED_URI);
    expect(asset.uri).not.toBe(NFT_URI);
  });

  it("serves the updated metadata document", async () => {
    const response = await fetch(asset.uri);
    expect(response.ok).toBe(true);

    const metadata = (await response.json()) as { name: string; image: string };
    expect(metadata.name).toBe(NFT_UPDATED_NAME);
    expect(metadata.image).toMatch(/dodge-challenger\.png$/);
  });
});
