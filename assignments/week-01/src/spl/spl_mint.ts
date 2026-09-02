import { address } from "@solana/kit";
import {
  findAssociatedTokenPda,
  getCreateAssociatedTokenInstructionAsync,
  getMintToInstruction,
  TOKEN_PROGRAM_ADDRESS,
} from "@solana-program/token";
import { requireArtifact, writeArtifacts } from "../lib/artifacts";
import { MINT_AMOUNT } from "../lib/config";
import { explorer } from "../lib/rpc";
import { sendInstructions } from "../lib/tx";
import { loadSigner } from "../lib/wallet";

(async () => {
  const signer = await loadSigner();
  const mint = address(requireArtifact("mint"));

  const [ata] = await findAssociatedTokenPda({
    mint,
    owner: signer.address,
    tokenProgram: TOKEN_PROGRAM_ADDRESS,
  });

  const createAtaIx = await getCreateAssociatedTokenInstructionAsync({
    payer: signer,
    owner: signer.address,
    mint,
  });

  const mintToIx = getMintToInstruction({
    mint,
    token: ata,
    mintAuthority: signer,
    amount: MINT_AMOUNT,
  });

  const signature = await sendInstructions(signer, [createAtaIx, mintToIx]);

  writeArtifacts({
    ownerAta: ata,
    mintedAmount: MINT_AMOUNT.toString(),
    signatures: { splMint: signature },
  });

  console.log(`ata       ${ata}`);
  console.log(`minted    ${MINT_AMOUNT}`);
  console.log(`signature ${signature}`);
  console.log(explorer("tx", signature));
})();
