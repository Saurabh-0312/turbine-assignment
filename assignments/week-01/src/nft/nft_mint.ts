import { create } from "@metaplex-foundation/mpl-core";
import { generateSigner } from "@metaplex-foundation/umi";
import bs58 from "bs58";
import { writeArtifacts } from "../lib/artifacts";
import { NFT_NAME, NFT_URI } from "../lib/config";
import { createUmiClient, explorer } from "../lib/rpc";

(async () => {
  const umi = createUmiClient();
  const asset = generateSigner(umi);

  const result = await create(umi, {
    asset,
    name: NFT_NAME,
    uri: NFT_URI,
    owner: umi.identity.publicKey,
  }).sendAndConfirm(umi);

  const signature = bs58.encode(result.signature);

  writeArtifacts({
    asset: asset.publicKey,
    nftName: NFT_NAME,
    nftUri: NFT_URI,
    signatures: { nftMint: signature },
  });

  console.log(`asset     ${asset.publicKey}`);
  console.log(`name      ${NFT_NAME}`);
  console.log(`uri       ${NFT_URI}`);
  console.log(`owner     ${umi.identity.publicKey}`);
  console.log(`signature ${signature}`);
  console.log(explorer("address", asset.publicKey));
})();
