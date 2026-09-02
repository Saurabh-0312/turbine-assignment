import { none, publicKey } from "@metaplex-foundation/umi";
import {
  createMetadataAccountV3,
  type CreateMetadataAccountV3InstructionArgs,
  type DataV2Args,
} from "@metaplex-foundation/mpl-token-metadata";
import bs58 from "bs58";
import { requireArtifact, writeArtifacts } from "../lib/artifacts";
import { TOKEN_NAME, TOKEN_SYMBOL, TOKEN_URI } from "../lib/config";
import { createUmiClient, explorer } from "../lib/rpc";

(async () => {
  const umi = createUmiClient();
  const mint = publicKey(requireArtifact("mint"));

  const data: DataV2Args = {
    name: TOKEN_NAME,
    symbol: TOKEN_SYMBOL,
    uri: TOKEN_URI,
    sellerFeeBasisPoints: 0,
    creators: none(),
    collection: none(),
    uses: none(),
  };

  const args: CreateMetadataAccountV3InstructionArgs = {
    data,
    isMutable: true,
    collectionDetails: none(),
  };

  const result = await createMetadataAccountV3(umi, {
    mint,
    mintAuthority: umi.identity,
    ...args,
  }).sendAndConfirm(umi);

  const signature = bs58.encode(result.signature);

  writeArtifacts({
    tokenName: TOKEN_NAME,
    tokenSymbol: TOKEN_SYMBOL,
    tokenUri: TOKEN_URI,
    signatures: { splMetadata: signature },
  });

  console.log(`name      ${TOKEN_NAME}`);
  console.log(`symbol    ${TOKEN_SYMBOL}`);
  console.log(`uri       ${TOKEN_URI}`);
  console.log(`signature ${signature}`);
  console.log(explorer("tx", signature));
})();
