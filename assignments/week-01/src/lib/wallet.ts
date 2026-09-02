import { createKeyPairSignerFromBytes, type KeyPairSigner } from "@solana/kit";
import { existsSync, readFileSync } from "fs";
import { homedir } from "os";
import { join } from "path";

const DEFAULT_WALLET = join(homedir(), ".config", "solana", "id.json");

export function walletPath(): string {
  const configured = process.env.WALLET_PATH ?? DEFAULT_WALLET;
  if (!existsSync(configured)) {
    throw new Error(`wallet not found at ${configured}, set WALLET_PATH to override`);
  }
  return configured;
}

export function loadSecretKey(): Uint8Array {
  return new Uint8Array(JSON.parse(readFileSync(walletPath(), "utf8")));
}

export async function loadSigner(): Promise<KeyPairSigner> {
  return createKeyPairSignerFromBytes(loadSecretKey());
}
