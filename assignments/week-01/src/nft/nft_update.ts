import { fetchAsset, update, type AssetV1 } from "@metaplex-foundation/mpl-core";
import { publicKey, type PublicKey, type Umi } from "@metaplex-foundation/umi";
import bs58 from "bs58";
import { requireArtifact, writeArtifacts } from "../lib/artifacts";
import { NFT_UPDATED_NAME, NFT_UPDATED_URI } from "../lib/config";
import { createUmiClient, explorer } from "../lib/rpc";

const RETRIES = 15;
const DELAY_MS = 1000;

const sleep = (ms: number) => new Promise((resolve) => setTimeout(resolve, ms));

async function fetchAssetWhen(
  umi: Umi,
  asset: PublicKey,
  matches: (value: AssetV1) => boolean,
): Promise<AssetV1> {
  let lastError: unknown;

  for (let attempt = 0; attempt < RETRIES; attempt += 1) {
    try {
      const value = await fetchAsset(umi, asset);
      if (matches(value)) return value;
    } catch (error) {
      lastError = error;
    }
    await sleep(DELAY_MS);
  }

  throw lastError ?? new Error(`asset ${asset} did not reach the expected state`);
}

(async () => {
  const umi = createUmiClient();
  const assetAddress = publicKey(requireArtifact("asset"));

  const before = await fetchAssetWhen(umi, assetAddress, () => true);

  const result = await update(umi, {
    asset: before,
    name: NFT_UPDATED_NAME,
    uri: NFT_UPDATED_URI,
  }).sendAndConfirm(umi);

  const signature = bs58.encode(result.signature);

  const after = await fetchAssetWhen(
    umi,
    assetAddress,
    (value) => value.name === NFT_UPDATED_NAME && value.uri === NFT_UPDATED_URI,
  );

  writeArtifacts({
    nftUpdatedName: after.name,
    nftUpdatedUri: after.uri,
    signatures: { nftUpdate: signature },
  });

  console.log(`asset     ${assetAddress}`);
  console.log(`name      ${before.name} -> ${after.name}`);
  console.log(`uri       ${before.uri} -> ${after.uri}`);
  console.log(`authority ${umi.identity.publicKey}`);
  console.log(`signature ${signature}`);
  console.log(explorer("address", assetAddress));
})();
