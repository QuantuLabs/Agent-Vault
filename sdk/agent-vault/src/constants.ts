import { PublicKey } from "@solana/web3.js";

export const AGENT_VAULT_PROGRAM_ID = new PublicKey("36u7KMBuxjExvU6V2nfTX5SnNdYMGUupFiYouLzrgpfW");
export const TOKEN_PROGRAM_ID = new PublicKey("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA");
export const TOKEN_2022_PROGRAM_ID = new PublicKey("TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb");
export const ASSOCIATED_TOKEN_PROGRAM_ID = new PublicKey("ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL");
export const NATIVE_MINT_ID = new PublicKey("So11111111111111111111111111111111111111112");

export const AGENT_VAULT_SEEDS = {
  globalConfig: "global_config",
  vaultConfig: "vault_config",
  agentWallet: "agent_vault",
} as const;

export const AGENT_VAULT_TAGS = {
  initializeGlobalConfig: 0,
  initVaultConfig: 1,
  createWallet: 2,
  updateWalletLabel: 3,
  depositSol: 4,
  withdrawSol: 5,
  transferSol: 6,
  closeWallet: 7,
  reopenWalletForRecovery: 8,
  createWalletAta: 32,
  transferSpl: 33,
  wrapSol: 34,
  unwrapSol: 35,
  closeWalletAta: 36,
  executeCpiChecked: 64,
} as const;

export const LABEL_LENGTH = 16;
export const GLOBAL_CONFIG_LENGTH = 160;
export const VAULT_CONFIG_LENGTH = 24;
export const WALLET_LENGTH = 32;

export const TOKEN_PROGRAM_KIND = {
  tokenkeg: 0,
  token2022: 1,
} as const;

export const DEVNET_RELEASE_MANIFEST = {
  schema: "agent-vault.release-manifest.v0",
  name: "Agent Vault",
  release: "devnet-v0.1.0-candidate",
  cluster: "devnet",
  deploymentStatus: "candidate-not-deployed",
  program: {
    id: AGENT_VAULT_PROGRAM_ID.toBase58(),
    globalConfigPda: "Fv7ffwFuAZBiCZ6dpBPKEgYEGMXpSArmqvaqfH35Gbod",
    globalConfigBump: 255,
    sbfElfSha256: "f91e7d5a435dfb741ad2bd04c3c0aa35c344199dde139299459b24f96cc0357b",
    sbfElfSizeBytes: 148120,
  },
  expectedGlobalConfig: {
    initializer: "2KmHw8VbShuz9xfj3ecEjBM5nPKR5BcYHRDSFfK1286t",
    registryProgram: "8oo4J9tBB3Hna1jRQ3rWvJjojqM5DYTDJo5cejUuJy3C",
    collection: "6CTyGPcn8dMwKEqgtvx2XCpkGUd7uqCVK6937RSM5bhA",
    feeTreasury: "EbHMHsePB6GYxjqgz9k2aC4NACx63vTeBXzXyHWFvqPK",
    vaultActivationFeeLamports: 500_000,
  },
} as const;
