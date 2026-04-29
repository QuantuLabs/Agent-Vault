import {
  PublicKey,
  SYSVAR_CLOCK_PUBKEY,
  SYSVAR_RENT_PUBKEY,
  SystemProgram,
  TransactionInstruction,
  type AccountMeta,
} from "@solana/web3.js";
import {
  AGENT_VAULT_PROGRAM_ID,
  AGENT_VAULT_TAGS,
  ASSOCIATED_TOKEN_PROGRAM_ID,
  DEVNET_RELEASE_MANIFEST,
  NATIVE_MINT_ID,
  TOKEN_2022_PROGRAM_ID,
  TOKEN_PROGRAM_ID,
  TOKEN_PROGRAM_KIND,
} from "./constants.js";
import { concatData, encodeLabel, toPublicKey, u16Le, u64Le } from "./codec.js";
import { AgentVaultPdas } from "./pda.js";
import type { AgentVaultReleaseManifest, ExecuteCpiCheckedParams, PublicKeyish, TransferSplParams, U64Input } from "./types.js";

export class AgentVaultInstructions {
  readonly pdas: AgentVaultPdas;
  readonly programId: PublicKey;
  readonly releaseManifest: AgentVaultReleaseManifest;

  constructor(params: {
    programId?: PublicKeyish;
    registryProgram?: PublicKeyish;
    releaseManifest?: AgentVaultReleaseManifest;
  } = {}) {
    this.programId = params.programId ? toPublicKey(params.programId) : AGENT_VAULT_PROGRAM_ID;
    this.releaseManifest = params.releaseManifest ?? DEVNET_RELEASE_MANIFEST;
    const registryProgram = params.registryProgram ?? this.releaseManifest.expectedGlobalConfig.registryProgram;
    this.pdas = new AgentVaultPdas(this.programId, toPublicKey(registryProgram));
  }

  initializeGlobalConfig(params: {
    initializer: PublicKeyish;
    registryProgram?: PublicKeyish;
    collection?: PublicKeyish;
    feeTreasury?: PublicKeyish;
    vaultActivationFeeLamports?: U64Input;
  }): TransactionInstruction {
    const registryProgram = toPublicKey(params.registryProgram ?? this.releaseManifest.expectedGlobalConfig.registryProgram);
    const collection = toPublicKey(params.collection ?? this.releaseManifest.expectedGlobalConfig.collection);
    const feeTreasury = toPublicKey(params.feeTreasury ?? this.releaseManifest.expectedGlobalConfig.feeTreasury);
    const fee = params.vaultActivationFeeLamports ?? this.releaseManifest.expectedGlobalConfig.vaultActivationFeeLamports;
    return new TransactionInstruction({
      programId: this.programId,
      keys: [
        meta(params.initializer, true, true),
        meta(this.pdas.globalConfig()[0], false, true),
        meta(SystemProgram.programId),
      ],
      data: concatData(
        AGENT_VAULT_TAGS.initializeGlobalConfig,
        registryProgram.toBuffer(),
        collection.toBuffer(),
        feeTreasury.toBuffer(),
        u64Le(fee),
      ),
    });
  }

  initVaultConfig(agentAsset: PublicKeyish, holder: PublicKeyish): TransactionInstruction {
    return new TransactionInstruction({
      programId: this.programId,
      keys: [
        meta(holder, true, true),
        meta(this.pdas.globalConfig()[0]),
        meta(this.pdas.vaultConfig(agentAsset)[0], false, true),
        meta(agentAsset),
        meta(this.pdas.agentAccount(agentAsset)[0]),
        meta(this.releaseManifest.expectedGlobalConfig.feeTreasury, false, true),
        meta(SYSVAR_CLOCK_PUBKEY),
        meta(SystemProgram.programId),
      ],
      data: Buffer.from([AGENT_VAULT_TAGS.initVaultConfig]),
    });
  }

  createWallet(agentAsset: PublicKeyish, holder: PublicKeyish, index: number, label?: string | Uint8Array): TransactionInstruction {
    return new TransactionInstruction({
      programId: this.programId,
      keys: [
        meta(holder, true, true),
        meta(this.pdas.vaultConfig(agentAsset)[0], false, true),
        meta(this.pdas.wallet(agentAsset, index)[0], false, true),
        meta(agentAsset),
        meta(SystemProgram.programId),
      ],
      data: concatData(AGENT_VAULT_TAGS.createWallet, encodeLabel(label)),
    });
  }

  updateWalletLabel(agentAsset: PublicKeyish, holder: PublicKeyish, index: number, label: string | Uint8Array): TransactionInstruction {
    return new TransactionInstruction({
      programId: this.programId,
      keys: [
        meta(holder, true),
        meta(this.pdas.globalConfig()[0]),
        meta(this.pdas.vaultConfig(agentAsset)[0]),
        meta(this.pdas.wallet(agentAsset, index)[0], false, true),
        meta(agentAsset),
      ],
      data: concatData(AGENT_VAULT_TAGS.updateWalletLabel, u16Le(index), encodeLabel(label)),
    });
  }

  depositSol(agentAsset: PublicKeyish, index: number, funder: PublicKeyish, amount: U64Input): TransactionInstruction {
    return new TransactionInstruction({
      programId: this.programId,
      keys: [
        meta(funder, true, true),
        meta(this.pdas.wallet(agentAsset, index)[0], false, true),
        meta(agentAsset),
        meta(SystemProgram.programId),
      ],
      data: concatData(AGENT_VAULT_TAGS.depositSol, u64Le(amount)),
    });
  }

  withdrawSol(agentAsset: PublicKeyish, holder: PublicKeyish, index: number, amount: U64Input, destination: PublicKeyish): TransactionInstruction {
    return new TransactionInstruction({
      programId: this.programId,
      keys: [
        meta(holder, true),
        meta(this.pdas.wallet(agentAsset, index)[0], false, true),
        meta(destination, false, true),
        meta(agentAsset),
        meta(SYSVAR_RENT_PUBKEY),
      ],
      data: concatData(AGENT_VAULT_TAGS.withdrawSol, u16Le(index), u64Le(amount)),
    });
  }

  transferSol(agentAsset: PublicKeyish, holder: PublicKeyish, fromIndex: number, toIndex: number, amount: U64Input): TransactionInstruction {
    return new TransactionInstruction({
      programId: this.programId,
      keys: [
        meta(holder, true),
        meta(this.pdas.wallet(agentAsset, fromIndex)[0], false, true),
        meta(this.pdas.wallet(agentAsset, toIndex)[0], false, true),
        meta(agentAsset),
        meta(SYSVAR_RENT_PUBKEY),
      ],
      data: concatData(AGENT_VAULT_TAGS.transferSol, u16Le(fromIndex), u16Le(toIndex), u64Le(amount)),
    });
  }

  closeWallet(agentAsset: PublicKeyish, holder: PublicKeyish, index: number, rentReceiver: PublicKeyish): TransactionInstruction {
    return new TransactionInstruction({
      programId: this.programId,
      keys: [
        meta(holder, true),
        meta(this.pdas.globalConfig()[0]),
        meta(this.pdas.vaultConfig(agentAsset)[0]),
        meta(this.pdas.wallet(agentAsset, index)[0], false, true),
        meta(rentReceiver, false, true),
        meta(agentAsset),
        meta(SYSVAR_RENT_PUBKEY),
      ],
      data: Buffer.from([AGENT_VAULT_TAGS.closeWallet]),
    });
  }

  reopenForRecovery(agentAsset: PublicKeyish, holder: PublicKeyish, index: number, label?: string | Uint8Array): TransactionInstruction {
    return new TransactionInstruction({
      programId: this.programId,
      keys: [
        meta(holder, true, true),
        meta(this.pdas.globalConfig()[0]),
        meta(this.pdas.vaultConfig(agentAsset)[0]),
        meta(this.pdas.wallet(agentAsset, index)[0], false, true),
        meta(agentAsset),
        meta(SystemProgram.programId),
      ],
      data: concatData(AGENT_VAULT_TAGS.reopenWalletForRecovery, u16Le(index), encodeLabel(label)),
    });
  }

  createAta(agentAsset: PublicKeyish, holder: PublicKeyish, index: number, mint: PublicKeyish, tokenProgram: PublicKeyish = TOKEN_PROGRAM_ID): TransactionInstruction {
    const wallet = this.pdas.wallet(agentAsset, index)[0];
    const token = toPublicKey(tokenProgram);
    return new TransactionInstruction({
      programId: this.programId,
      keys: [
        meta(holder, true, true),
        meta(this.pdas.globalConfig()[0]),
        meta(this.pdas.vaultConfig(agentAsset)[0]),
        meta(wallet),
        meta(agentAsset),
        meta(mint),
        meta(this.pdas.walletAta(wallet, mint, token)[0], false, true),
        meta(ASSOCIATED_TOKEN_PROGRAM_ID),
        meta(token),
        meta(SystemProgram.programId),
      ],
      data: concatData(AGENT_VAULT_TAGS.createWalletAta, u16Le(index), Buffer.from([tokenProgramKind(token)])),
    });
  }

  transferSpl(agentAsset: PublicKeyish, holder: PublicKeyish, index: number, params: TransferSplParams): TransactionInstruction {
    const token = toPublicKey(params.tokenProgram ?? TOKEN_PROGRAM_ID);
    return new TransactionInstruction({
      programId: this.programId,
      keys: [
        meta(holder, true),
        meta(this.pdas.globalConfig()[0]),
        meta(this.pdas.vaultConfig(agentAsset)[0]),
        meta(this.pdas.wallet(agentAsset, index)[0]),
        meta(agentAsset),
        meta(params.mint),
        meta(params.source, false, true),
        meta(params.destination, false, true),
        meta(token),
      ],
      data: concatData(
        AGENT_VAULT_TAGS.transferSpl,
        u16Le(index),
        u64Le(params.amount),
        Buffer.from([params.decimals]),
        u64Le(params.expectedFee ?? 0),
      ),
    });
  }

  wrapSol(agentAsset: PublicKeyish, holder: PublicKeyish, index: number, amount: U64Input): TransactionInstruction {
    const wallet = this.pdas.wallet(agentAsset, index)[0];
    return new TransactionInstruction({
      programId: this.programId,
      keys: [
        meta(holder, true, true),
        meta(this.pdas.globalConfig()[0]),
        meta(this.pdas.vaultConfig(agentAsset)[0]),
        meta(wallet, false, true),
        meta(agentAsset),
        meta(this.pdas.walletAta(wallet, NATIVE_MINT_ID, TOKEN_PROGRAM_ID)[0], false, true),
        meta(NATIVE_MINT_ID),
        meta(TOKEN_PROGRAM_ID),
        meta(SYSVAR_RENT_PUBKEY),
      ],
      data: concatData(AGENT_VAULT_TAGS.wrapSol, u16Le(index), u64Le(amount)),
    });
  }

  unwrapSol(agentAsset: PublicKeyish, holder: PublicKeyish, index: number): TransactionInstruction {
    const wallet = this.pdas.wallet(agentAsset, index)[0];
    return new TransactionInstruction({
      programId: this.programId,
      keys: [
        meta(holder, true),
        meta(this.pdas.globalConfig()[0]),
        meta(this.pdas.vaultConfig(agentAsset)[0]),
        meta(wallet, false, true),
        meta(agentAsset),
        meta(this.pdas.walletAta(wallet, NATIVE_MINT_ID, TOKEN_PROGRAM_ID)[0], false, true),
        meta(TOKEN_PROGRAM_ID),
      ],
      data: concatData(AGENT_VAULT_TAGS.unwrapSol, u16Le(index)),
    });
  }

  closeAta(agentAsset: PublicKeyish, holder: PublicKeyish, index: number, mint: PublicKeyish, tokenProgram: PublicKeyish, rentReceiver: PublicKeyish): TransactionInstruction {
    const wallet = this.pdas.wallet(agentAsset, index)[0];
    const token = toPublicKey(tokenProgram);
    return new TransactionInstruction({
      programId: this.programId,
      keys: [
        meta(holder, true),
        meta(this.pdas.globalConfig()[0]),
        meta(this.pdas.vaultConfig(agentAsset)[0]),
        meta(wallet),
        meta(agentAsset),
        meta(mint),
        meta(this.pdas.walletAta(wallet, mint, token)[0], false, true),
        meta(rentReceiver, false, true),
        meta(ASSOCIATED_TOKEN_PROGRAM_ID),
        meta(token),
      ],
      data: concatData(AGENT_VAULT_TAGS.closeWalletAta, u16Le(index)),
    });
  }

  executeCpiChecked(agentAsset: PublicKeyish, holder: PublicKeyish, index: number, params: ExecuteCpiCheckedParams): TransactionInstruction {
    const targetAccounts = params.targetAccounts.map((account) =>
      meta(account.pubkey, account.isSigner ?? false, account.isWritable ?? false)
    );
    return new TransactionInstruction({
      programId: this.programId,
      keys: [
        meta(holder, true),
        meta(this.pdas.globalConfig()[0]),
        meta(this.pdas.vaultConfig(agentAsset)[0]),
        meta(this.pdas.wallet(agentAsset, index)[0]),
        meta(agentAsset),
        meta(params.targetProgram),
        ...targetAccounts,
      ],
      data: concatData(
        AGENT_VAULT_TAGS.executeCpiChecked,
        u16Le(index),
        Buffer.from([params.walletMetaIndex, params.targetAccounts.length]),
        u16Le(params.targetInstructionData.length),
        params.targetInstructionData,
        Buffer.from([params.postCheckCount]),
        params.postCheckData,
      ),
    });
  }
}

function tokenProgramKind(tokenProgram: PublicKey): number {
  if (tokenProgram.equals(TOKEN_PROGRAM_ID)) {
    return TOKEN_PROGRAM_KIND.tokenkeg;
  }
  if (tokenProgram.equals(TOKEN_2022_PROGRAM_ID)) {
    return TOKEN_PROGRAM_KIND.token2022;
  }
  throw new Error("unsupported token program");
}

function meta(pubkey: PublicKeyish, isSigner = false, isWritable = false): AccountMeta {
  return {
    pubkey: toPublicKey(pubkey),
    isSigner,
    isWritable,
  };
}
