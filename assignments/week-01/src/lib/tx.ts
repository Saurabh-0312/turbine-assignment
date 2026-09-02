import {
  appendTransactionMessageInstructions,
  assertIsTransactionWithBlockhashLifetime,
  createTransactionMessage,
  getSignatureFromTransaction,
  setTransactionMessageFeePayerSigner,
  setTransactionMessageLifetimeUsingBlockhash,
  signTransactionMessageWithSigners,
  type KeyPairSigner,
} from "@solana/kit";
import { rpc, sendAndConfirm } from "./rpc";

type Instructions = Parameters<typeof appendTransactionMessageInstructions>[0];

export async function sendInstructions(
  signer: KeyPairSigner,
  instructions: Instructions,
): Promise<string> {
  const { value: latestBlockhash } = await rpc.getLatestBlockhash().send();

  const message = appendTransactionMessageInstructions(
    instructions,
    setTransactionMessageLifetimeUsingBlockhash(
      latestBlockhash,
      setTransactionMessageFeePayerSigner(signer, createTransactionMessage({ version: 0 })),
    ),
  );

  const signed = await signTransactionMessageWithSigners(message);
  assertIsTransactionWithBlockhashLifetime(signed);

  const signature = getSignatureFromTransaction(signed);
  await sendAndConfirm(signed, { commitment: "confirmed" });

  return signature;
}
