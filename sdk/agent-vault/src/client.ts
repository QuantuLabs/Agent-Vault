import { AgentVaultIdentitiesClient } from "./identities.js";
import { AgentVaultWalletsClient } from "./wallets.js";
import { toPublicKey } from "./codec.js";
import { AGENT_VAULT_PROGRAM_ID, DEVNET_RELEASE_MANIFEST } from "./constants.js";
import type { AgentVaultClientConfig } from "./types.js";

export class AgentVaultClient {
  readonly identities: AgentVaultIdentitiesClient;
  readonly wallets: AgentVaultWalletsClient;

  constructor(readonly config: AgentVaultClientConfig) {
    const releaseManifest = config.releaseManifest ?? DEVNET_RELEASE_MANIFEST;
    const programId = config.programId ? toPublicKey(config.programId) : AGENT_VAULT_PROGRAM_ID;
    const registryProgram = config.registryProgram
      ? toPublicKey(config.registryProgram)
      : toPublicKey(releaseManifest.expectedGlobalConfig.registryProgram);

    this.wallets = new AgentVaultWalletsClient(config.connection, {
      programId,
      registryProgram,
      releaseManifest,
    });
    this.identities = new AgentVaultIdentitiesClient(this.wallets.pdas, config.identity);
  }
}

export function createAgentVaultClient(config: AgentVaultClientConfig): AgentVaultClient {
  return new AgentVaultClient(config);
}
