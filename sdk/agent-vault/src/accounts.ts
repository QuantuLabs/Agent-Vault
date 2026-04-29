import { PublicKey } from "@solana/web3.js";
import { decodeLabel } from "./codec.js";
import {
  GLOBAL_CONFIG_LENGTH,
  VAULT_CONFIG_LENGTH,
  WALLET_LENGTH,
} from "./constants.js";
import type { AgentVaultReleaseManifest, AgentWalletAccount, VaultConfig } from "./types.js";

const GLOBAL_CONFIG_INITIALIZER_OFFSET = 10;
const GLOBAL_CONFIG_REGISTRY_PROGRAM_OFFSET = 42;
const GLOBAL_CONFIG_COLLECTION_OFFSET = 74;
const GLOBAL_CONFIG_FEE_TREASURY_OFFSET = 106;
const GLOBAL_CONFIG_FEE_OFFSET = 138;
const VAULT_CONFIG_WALLET_COUNT_OFFSET = 10;
const VAULT_CONFIG_CREATED_AT_OFFSET = 14;
const WALLET_INDEX_OFFSET = 10;
const WALLET_FLAGS_OFFSET = 12;
const WALLET_LABEL_OFFSET = 14;
const WALLET_FLAG_ACTIVE = 1 << 0;
const WALLET_FLAG_RECOVERY_ONLY = 1 << 1;

export interface GlobalConfigAccount {
  bump: number;
  initializer: PublicKey;
  registryProgram: PublicKey;
  collection: PublicKey;
  feeTreasury: PublicKey;
  vaultActivationFeeLamports: bigint;
}

export function parseGlobalConfig(data: Buffer): GlobalConfigAccount {
  requireLength(data, GLOBAL_CONFIG_LENGTH, "global config");
  return {
    bump: data[9] ?? 0,
    initializer: new PublicKey(data.subarray(GLOBAL_CONFIG_INITIALIZER_OFFSET, GLOBAL_CONFIG_INITIALIZER_OFFSET + 32)),
    registryProgram: new PublicKey(data.subarray(GLOBAL_CONFIG_REGISTRY_PROGRAM_OFFSET, GLOBAL_CONFIG_REGISTRY_PROGRAM_OFFSET + 32)),
    collection: new PublicKey(data.subarray(GLOBAL_CONFIG_COLLECTION_OFFSET, GLOBAL_CONFIG_COLLECTION_OFFSET + 32)),
    feeTreasury: new PublicKey(data.subarray(GLOBAL_CONFIG_FEE_TREASURY_OFFSET, GLOBAL_CONFIG_FEE_TREASURY_OFFSET + 32)),
    vaultActivationFeeLamports: data.readBigUInt64LE(GLOBAL_CONFIG_FEE_OFFSET),
  };
}

export function parseVaultConfig(address: PublicKey, data: Buffer): VaultConfig {
  requireLength(data, VAULT_CONFIG_LENGTH, "vault config");
  return {
    address,
    bump: data[9] ?? 0,
    walletCount: data.readUInt16LE(VAULT_CONFIG_WALLET_COUNT_OFFSET),
    createdAt: data.readBigInt64LE(VAULT_CONFIG_CREATED_AT_OFFSET),
  };
}

export function parseWallet(data: Buffer): AgentWalletAccount {
  requireLength(data, WALLET_LENGTH, "wallet");
  const flags = data.readUInt16LE(WALLET_FLAGS_OFFSET);
  return {
    bump: data[9] ?? 0,
    index: data.readUInt16LE(WALLET_INDEX_OFFSET),
    flags,
    label: decodeLabel(data.subarray(WALLET_LABEL_OFFSET, WALLET_LABEL_OFFSET + 16)),
    isActive: (flags & WALLET_FLAG_ACTIVE) !== 0,
    isRecoveryOnly: (flags & WALLET_FLAG_RECOVERY_ONLY) !== 0,
  };
}

export function compareGlobalConfigToManifest(
  account: GlobalConfigAccount,
  manifest: AgentVaultReleaseManifest,
): string[] {
  const expected = manifest.expectedGlobalConfig;
  const issues: string[] = [];
  compare("initializer", account.initializer.toBase58(), expected.initializer, issues);
  compare("registryProgram", account.registryProgram.toBase58(), expected.registryProgram, issues);
  compare("collection", account.collection.toBase58(), expected.collection, issues);
  compare("feeTreasury", account.feeTreasury.toBase58(), expected.feeTreasury, issues);
  compare(
    "vaultActivationFeeLamports",
    account.vaultActivationFeeLamports.toString(),
    String(expected.vaultActivationFeeLamports),
    issues,
  );
  return issues;
}

function compare(name: string, actual: string, expected: string, issues: string[]): void {
  if (actual !== expected) {
    issues.push(`${name} mismatch: expected ${expected}, got ${actual}`);
  }
}

function requireLength(data: Buffer, expected: number, label: string): void {
  if (data.length !== expected) {
    throw new Error(`invalid ${label} length: expected ${expected}, got ${data.length}`);
  }
}
