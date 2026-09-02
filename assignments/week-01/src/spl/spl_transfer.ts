import { address, generateKeyPairSigner } from "@solana/kit";
import {
  findAssociatedTokenPda,
  getCreateAssociatedTokenInstructionAsync,
  getTransferCheckedInstruction,
  TOKEN_PROGRAM_ADDRESS,
} from "@solana-program/token";
import { readArtifacts, requireArtifact, writeArtifacts } from "../lib/artifacts";
import { TOKEN_DECIMALS, TRANSFER_AMOUNT } from "../lib/config";
import { explorer } from "../lib/rpc";
import { sendInstructions } from "../lib/tx";
import { loadSigner } from "../lib/wallet";

(async () => {
  const signer = await loadSigner();
  const mint = address(requireArtifact("mint"));

  const saved = readArtifacts().recipient;
  const recipient = saved ? address(saved) : (await generateKeyPairSigner()).address;

  const [fromAta] = await findAssociatedTokenPda({
    mint,
    owner: signer.address,
    tokenProgram: TOKEN_PROGRAM_ADDRESS,
  });

  const [toAta] = await findAssociatedTokenPda({
    mint,
    owner: recipient,
    tokenProgram: TOKEN_PROGRAM_ADDRESS,
  });

  const createAtaIx = await getCreateAssociatedTokenInstructionAsync({
    payer: signer,
    owner: recipient,
    mint,
  });

  const transferIx = getTransferCheckedInstruction({
    source: fromAta,
    mint,
    destination: toAta,
    authority: signer,
    amount: TRANSFER_AMOUNT,
    decimals: TOKEN_DECIMALS,
  });

  const signature = await sendInstructions(signer, [createAtaIx, transferIx]);

  writeArtifacts({
    recipient,
    recipientAta: toAta,
    transferredAmount: TRANSFER_AMOUNT.toString(),
    signatures: { splTransfer: signature },
  });

  console.log(`from      ${fromAta}`);
  console.log(`to        ${toAta}`);
  console.log(`recipient ${recipient}`);
  console.log(`amount    ${TRANSFER_AMOUNT}`);
  console.log(`signature ${signature}`);
  console.log(explorer("tx", signature));
})();
