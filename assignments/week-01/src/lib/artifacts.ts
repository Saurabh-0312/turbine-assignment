import { existsSync, readFileSync, writeFileSync } from "fs";
import { join } from "path";

const FILE = join(__dirname, "..", "..", "artifacts.json");

export type Artifacts = {
  mint?: string;
  mintDecimals?: number;
  ownerAta?: string;
  mintedAmount?: string;
  tokenName?: string;
  tokenSymbol?: string;
  tokenUri?: string;
  recipient?: string;
  recipientAta?: string;
  transferredAmount?: string;
  asset?: string;
  nftName?: string;
  nftUri?: string;
  nftUpdatedName?: string;
  nftUpdatedUri?: string;
  signatures?: Record<string, string>;
};

export function readArtifacts(): Artifacts {
  if (!existsSync(FILE)) return {};
  return JSON.parse(readFileSync(FILE, "utf8")) as Artifacts;
}

export function writeArtifacts(patch: Artifacts): Artifacts {
  const current = readArtifacts();
  const next: Artifacts = {
    ...current,
    ...patch,
    signatures: { ...current.signatures, ...patch.signatures },
  };
  writeFileSync(FILE, `${JSON.stringify(next, null, 2)}\n`);
  return next;
}

export function requireArtifact<K extends keyof Artifacts>(key: K): NonNullable<Artifacts[K]> {
  const value = readArtifacts()[key];
  if (value === undefined || value === null) {
    throw new Error(`missing artifact "${String(key)}", run the earlier scripts first`);
  }
  return value as NonNullable<Artifacts[K]>;
}
