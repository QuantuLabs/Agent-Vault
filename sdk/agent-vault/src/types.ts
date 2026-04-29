import type { AccountInfo, Connection, PublicKey, TransactionInstruction } from "@solana/web3.js";
import type { SolanaSDK } from "8004-solana";

export type PublicKeyish = PublicKey | string;
export type U64Input = bigint | number | string;
export type TokenProgramKind = "tokenkeg" | "token2022";

export type AgentVaultCluster = "devnet" | "mainnet" | "localnet";

export interface AgentVaultReleaseManifest {
  schema: "agent-vault.release-manifest.v0";
  name: "Agent Vault";
  release: string;
  cluster: AgentVaultCluster;
  deploymentStatus: "candidate-not-deployed" | "deployed";
  program: {
    id: string;
    globalConfigPda: string;
    globalConfigBump: number;
    sbfElfSha256: string;
    sbfElfSizeBytes: number;
  };
  expectedGlobalConfig: {
    initializer: string;
    registryProgram: string;
    collection: string;
    feeTreasury: string;
    vaultActivationFeeLamports: number;
  };
}

export type IdentitySdk = Pick<SolanaSDK, "registerAgent">;

export interface AgentVaultClientConfig {
  connection: Connection;
  programId?: PublicKeyish;
  registryProgram?: PublicKeyish;
  releaseManifest?: AgentVaultReleaseManifest;
  identity?: IdentitySdk;
}

export interface CreateIdentityParams {
  uri?: string;
  atomEnabled?: boolean;
  collectionPointer?: string;
  collectionLock?: boolean;
  skipSend?: boolean;
  signer?: PublicKey;
  assetPubkey?: PublicKey;
  options?: Record<string, unknown>;
}

export interface CreateIdentityResult {
  agentAsset: PublicKey;
  result: unknown;
}

export interface VaultConfig {
  address: PublicKey;
  bump: number;
  walletCount: number;
  createdAt: bigint;
}

export interface AgentWalletAccount {
  bump: number;
  index: number;
  flags: number;
  label: string;
  isActive: boolean;
  isRecoveryOnly: boolean;
}

export type WalletDataStatus = "active" | "recovery" | "closed" | "dusted" | "invalid";

export interface WalletRecord {
  agentAsset: PublicKey;
  index: number;
  address: PublicKey;
  exists: boolean;
  dataStatus: WalletDataStatus;
  label: string | null;
  lamports: number;
  account: AgentWalletAccount | null;
  rawAccount: AccountInfo<Buffer> | null;
}

export interface ListWalletsOptions {
  startIndex?: number;
  limit?: number;
  includeClosed?: boolean;
  chunkSize?: number;
}

export interface CreateWalletOptions {
  label?: string | Uint8Array;
  index?: number;
}

export interface TransferSplParams {
  mint: PublicKeyish;
  source: PublicKeyish;
  destination: PublicKeyish;
  amount: U64Input;
  decimals: number;
  tokenProgram?: PublicKeyish;
  expectedFee?: U64Input;
}

export interface ExecuteCpiCheckedParams {
  walletMetaIndex: number;
  targetProgram: PublicKeyish;
  targetAccounts: Array<{
    pubkey: PublicKeyish;
    isSigner?: boolean;
    isWritable?: boolean;
  }>;
  targetInstructionData: Uint8Array;
  postCheckCount: number;
  postCheckData: Uint8Array;
}

export interface DeploymentVerification {
  ok: boolean;
  status: "verified" | "missing" | "mismatch";
  issues: string[];
}

export interface WalletInstructionSet {
  initializeGlobalConfig(params: {
    initializer: PublicKeyish;
    registryProgram: PublicKeyish;
    collection: PublicKeyish;
    feeTreasury: PublicKeyish;
    vaultActivationFeeLamports: U64Input;
  }): TransactionInstruction;
}
