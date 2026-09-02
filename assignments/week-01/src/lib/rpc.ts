import {
  createSolanaRpc,
  createSolanaRpcSubscriptions,
  sendAndConfirmTransactionFactory,
} from "@solana/kit";
import { createSignerFromKeypair, signerIdentity, type Umi } from "@metaplex-foundation/umi";
import { createUmi } from "@metaplex-foundation/umi-bundle-defaults";
import { loadSecretKey } from "./wallet";

export const DEVNET_HTTP = process.env.RPC_URL ?? "https://api.devnet.solana.com";
export const DEVNET_WS = process.env.RPC_WS ?? "wss://api.devnet.solana.com";

export const rpc = createSolanaRpc(DEVNET_HTTP);
export const rpcSubscriptions = createSolanaRpcSubscriptions(DEVNET_WS);
export const sendAndConfirm = sendAndConfirmTransactionFactory({ rpc, rpcSubscriptions });

export function explorer(kind: "tx" | "address", value: string): string {
  return `https://explorer.solana.com/${kind}/${value}?cluster=devnet`;
}

export function createUmiClient(): Umi {
  const umi = createUmi(DEVNET_HTTP);
  const keypair = umi.eddsa.createKeypairFromSecretKey(loadSecretKey());
  umi.use(signerIdentity(createSignerFromKeypair(umi, keypair)));
  return umi;
}
