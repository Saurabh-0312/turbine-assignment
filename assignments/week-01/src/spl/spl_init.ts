import { generateKeyPairSigner } from "@solana/kit";
import { getCreateAccountInstruction } from "@solana-program/system";
import {
  getInitializeMintInstruction,
  getMintSize,
  TOKEN_PROGRAM_ADDRESS,
} from "@solana-program/token";
import { writeArtifacts } from "../lib/artifacts";
import { TOKEN_DECIMALS } from "../lib/config";
import { explorer, rpc } from "../lib/rpc";
import { sendInstructions } from "../lib/tx";
import { loadSigner } from "../lib/wallet";

(async () => {
  const signer = await loadSigner();
  const mint = await generateKeyPairSigner();

  const space = BigInt(getMintSize());
  const rent = await rpc.getMinimumBalanceForRentExemption(space).send();

  const createAccountIx = getCreateAccountInstruction({
    payer: signer,
    newAccount: mint,
    lamports: rent,
    space,
    programAddress: TOKEN_PROGRAM_ADDRESS,
  });

  const initializeMintIx = getInitializeMintInstruction({
    mint: mint.address,
    decimals: TOKEN_DECIMALS,
    mintAuthority: signer.address,
    freezeAuthority: signer.address,
  });

  const signature = await sendInstructions(signer, [createAccountIx, initializeMintIx]);

  writeArtifacts({
    mint: mint.address,
    mintDecimals: TOKEN_DECIMALS,
    signatures: { splInit: signature },
  });

  console.log(`mint      ${mint.address}`);
  console.log(`decimals  ${TOKEN_DECIMALS}`);
  console.log(`signature ${signature}`);
  console.log(explorer("address", mint.address));
})();
