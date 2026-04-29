import { PublicKey } from "@solana/web3.js";
import { PDAHelpers } from "8004-solana";
import { AGENT_VAULT_PROGRAM_ID, AGENT_VAULT_SEEDS, ASSOCIATED_TOKEN_PROGRAM_ID } from "./constants.js";
import { toPublicKey, u16Le } from "./codec.js";
import type { PublicKeyish } from "./types.js";

export class AgentVaultPdas {
  constructor(
    readonly programId: PublicKey = AGENT_VAULT_PROGRAM_ID,
    readonly registryProgram?: PublicKey,
  ) {}

  globalConfig(): [PublicKey, number] {
    return PublicKey.findProgramAddressSync([Buffer.from(AGENT_VAULT_SEEDS.globalConfig)], this.programId);
  }

  vaultConfig(agentAsset: PublicKeyish): [PublicKey, number] {
    const asset = toPublicKey(agentAsset);
    return PublicKey.findProgramAddressSync(
      [Buffer.from(AGENT_VAULT_SEEDS.vaultConfig), asset.toBuffer()],
      this.programId,
    );
  }

  wallet(agentAsset: PublicKeyish, index: number): [PublicKey, number] {
    const asset = toPublicKey(agentAsset);
    return PublicKey.findProgramAddressSync(
      [Buffer.from(AGENT_VAULT_SEEDS.agentWallet), asset.toBuffer(), u16Le(index)],
      this.programId,
    );
  }

  walletAta(owner: PublicKeyish, mint: PublicKeyish, tokenProgram: PublicKeyish): [PublicKey, number] {
    return PublicKey.findProgramAddressSync(
      [toPublicKey(owner).toBuffer(), toPublicKey(tokenProgram).toBuffer(), toPublicKey(mint).toBuffer()],
      ASSOCIATED_TOKEN_PROGRAM_ID,
    );
  }

  agentAccount(agentAsset: PublicKeyish): [PublicKey, number] {
    if (!this.registryProgram) {
      throw new Error("registryProgram is required to derive an 8004 AgentAccount PDA");
    }
    return PDAHelpers.getAgentPDA(toPublicKey(agentAsset), this.registryProgram);
  }
}

export function createPdas(programId?: PublicKeyish, registryProgram?: PublicKeyish): AgentVaultPdas {
  return new AgentVaultPdas(
    programId ? toPublicKey(programId) : AGENT_VAULT_PROGRAM_ID,
    registryProgram ? toPublicKey(registryProgram) : undefined,
  );
}
