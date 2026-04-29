import { PublicKey, SystemProgram, type AccountInfo, type Connection, type TransactionInstruction } from "@solana/web3.js";
import { compareGlobalConfigToManifest, parseGlobalConfig, parseVaultConfig, parseWallet } from "./accounts.js";
import { DEVNET_RELEASE_MANIFEST, WALLET_LENGTH } from "./constants.js";
import { AgentVaultInstructions } from "./instructions.js";
import { toPublicKey } from "./codec.js";
import type {
  AgentVaultReleaseManifest,
  CreateWalletOptions,
  DeploymentVerification,
  ExecuteCpiCheckedParams,
  ListWalletsOptions,
  PublicKeyish,
  TransferSplParams,
  U64Input,
  VaultConfig,
  WalletRecord,
} from "./types.js";

const DEFAULT_LIST_LIMIT = 100;
const DEFAULT_CHUNK_SIZE = 100;

export class AgentVaultWalletsClient {
  readonly instructions: AgentVaultInstructions;

  constructor(
    private readonly connection: Connection,
    params: {
      programId?: PublicKeyish;
      registryProgram?: PublicKeyish;
      releaseManifest?: AgentVaultReleaseManifest;
    } = {},
  ) {
    this.instructions = new AgentVaultInstructions(params);
  }

  get pdas() {
    return this.instructions.pdas;
  }

  async getVault(agentAsset: PublicKeyish): Promise<VaultConfig | null> {
    const address = this.pdas.vaultConfig(agentAsset)[0];
    const info = await this.connection.getAccountInfo(address);
    if (!info) {
      return null;
    }
    return parseVaultConfig(address, Buffer.from(info.data));
  }

  async get(agentAsset: PublicKeyish, index: number): Promise<WalletRecord> {
    const asset = toPublicKey(agentAsset);
    const address = this.pdas.wallet(asset, index)[0];
    const info = await this.connection.getAccountInfo(address);
    return this.recordFromAccount(asset, index, address, info);
  }

  async list(agentAsset: PublicKeyish, options: ListWalletsOptions = {}): Promise<WalletRecord[]> {
    const asset = toPublicKey(agentAsset);
    const vault = await this.getVault(asset);
    if (!vault) {
      return [];
    }
    const startIndex = options.startIndex ?? 0;
    const limit = options.limit ?? DEFAULT_LIST_LIMIT;
    const chunkSize = options.chunkSize ?? DEFAULT_CHUNK_SIZE;
    const endIndex = Math.min(vault.walletCount, startIndex + limit);
    const records: WalletRecord[] = [];

    for (let cursor = startIndex; cursor < endIndex; cursor += chunkSize) {
      const chunkEnd = Math.min(endIndex, cursor + chunkSize);
      const addresses: PublicKey[] = [];
      for (let index = cursor; index < chunkEnd; index += 1) {
        addresses.push(this.pdas.wallet(asset, index)[0]);
      }
      const accounts = await this.connection.getMultipleAccountsInfo(addresses);
      for (let offset = 0; offset < accounts.length; offset += 1) {
        const index = cursor + offset;
        const address = addresses[offset];
        if (!address) {
          continue;
        }
        const record = this.recordFromAccount(asset, index, address, accounts[offset] ?? null);
        if (options.includeClosed || record.exists) {
          records.push(record);
        }
      }
    }

    return records;
  }

  async listAll(agentAsset: PublicKeyish, options: Omit<ListWalletsOptions, "limit"> = {}): Promise<WalletRecord[]> {
    const vault = await this.getVault(agentAsset);
    if (!vault) {
      return [];
    }
    const startIndex = options.startIndex ?? 0;
    return this.list(agentAsset, {
      ...options,
      limit: Math.max(0, vault.walletCount - startIndex),
    });
  }

  async create(agentAsset: PublicKeyish, holder: PublicKeyish, options: Omit<CreateWalletOptions, "index"> = {}): Promise<TransactionInstruction> {
    const vault = await this.requireVault(agentAsset);
    return this.buildCreate(agentAsset, holder, { ...options, index: vault.walletCount });
  }

  buildInitVault(agentAsset: PublicKeyish, holder: PublicKeyish): TransactionInstruction {
    return this.instructions.initVaultConfig(agentAsset, holder);
  }

  buildCreate(agentAsset: PublicKeyish, holder: PublicKeyish, options: CreateWalletOptions): TransactionInstruction {
    if (options.index === undefined) {
      throw new Error("options.index is required for buildCreate(); use create() to fetch wallet_count automatically");
    }
    return this.instructions.createWallet(agentAsset, holder, options.index, options.label);
  }

  buildUpdateLabel(agentAsset: PublicKeyish, holder: PublicKeyish, index: number, label: string | Uint8Array): TransactionInstruction {
    return this.instructions.updateWalletLabel(agentAsset, holder, index, label);
  }

  buildDepositSol(agentAsset: PublicKeyish, index: number, funder: PublicKeyish, amount: U64Input): TransactionInstruction {
    return this.instructions.depositSol(agentAsset, index, funder, amount);
  }

  buildWithdrawSol(agentAsset: PublicKeyish, holder: PublicKeyish, index: number, amount: U64Input, destination: PublicKeyish): TransactionInstruction {
    return this.instructions.withdrawSol(agentAsset, holder, index, amount, destination);
  }

  buildTransferSol(agentAsset: PublicKeyish, holder: PublicKeyish, fromIndex: number, toIndex: number, amount: U64Input): TransactionInstruction {
    return this.instructions.transferSol(agentAsset, holder, fromIndex, toIndex, amount);
  }

  buildCreateAta(agentAsset: PublicKeyish, holder: PublicKeyish, index: number, mint: PublicKeyish, tokenProgram?: PublicKeyish): TransactionInstruction {
    return tokenProgram
      ? this.instructions.createAta(agentAsset, holder, index, mint, tokenProgram)
      : this.instructions.createAta(agentAsset, holder, index, mint);
  }

  buildTransferSpl(agentAsset: PublicKeyish, holder: PublicKeyish, index: number, params: TransferSplParams): TransactionInstruction {
    return this.instructions.transferSpl(agentAsset, holder, index, params);
  }

  buildWrapSol(agentAsset: PublicKeyish, holder: PublicKeyish, index: number, amount: U64Input): TransactionInstruction {
    return this.instructions.wrapSol(agentAsset, holder, index, amount);
  }

  buildUnwrapSol(agentAsset: PublicKeyish, holder: PublicKeyish, index: number): TransactionInstruction {
    return this.instructions.unwrapSol(agentAsset, holder, index);
  }

  buildCloseAta(agentAsset: PublicKeyish, holder: PublicKeyish, index: number, mint: PublicKeyish, tokenProgram: PublicKeyish, rentReceiver: PublicKeyish): TransactionInstruction {
    return this.instructions.closeAta(agentAsset, holder, index, mint, tokenProgram, rentReceiver);
  }

  buildExecuteCpiChecked(agentAsset: PublicKeyish, holder: PublicKeyish, index: number, params: ExecuteCpiCheckedParams): TransactionInstruction {
    return this.instructions.executeCpiChecked(agentAsset, holder, index, params);
  }

  buildReopenForRecovery(agentAsset: PublicKeyish, holder: PublicKeyish, index: number, options: { label?: string | Uint8Array } = {}): TransactionInstruction {
    return this.instructions.reopenForRecovery(agentAsset, holder, index, options.label);
  }

  buildClose(agentAsset: PublicKeyish, holder: PublicKeyish, index: number, rentReceiver: PublicKeyish): TransactionInstruction {
    return this.instructions.closeWallet(agentAsset, holder, index, rentReceiver);
  }

  async verifyDeployment(manifest: AgentVaultReleaseManifest = DEVNET_RELEASE_MANIFEST): Promise<DeploymentVerification> {
    const expectedProgramId = new PublicKey(manifest.program.id);
    if (!expectedProgramId.equals(this.instructions.programId)) {
      return {
        ok: false,
        status: "mismatch",
        issues: [`program id mismatch: expected ${manifest.program.id}, got ${this.instructions.programId.toBase58()}`],
      };
    }

    const globalConfig = this.pdas.globalConfig()[0];
    const info = await this.connection.getAccountInfo(globalConfig);
    if (!info) {
      return {
        ok: false,
        status: "missing",
        issues: [`global config missing at ${globalConfig.toBase58()}`],
      };
    }
    const parsed = parseGlobalConfig(Buffer.from(info.data));
    const issues = compareGlobalConfigToManifest(parsed, manifest);
    return {
      ok: issues.length === 0,
      status: issues.length === 0 ? "verified" : "mismatch",
      issues,
    };
  }

  private async requireVault(agentAsset: PublicKeyish): Promise<VaultConfig> {
    const vault = await this.getVault(agentAsset);
    if (!vault) {
      throw new Error("vault config not found; initialize the Agent Vault first");
    }
    return vault;
  }

  private recordFromAccount(
    agentAsset: PublicKey,
    index: number,
    address: PublicKey,
    info: AccountInfo<Buffer> | null,
  ): WalletRecord {
    if (!info) {
      return {
        agentAsset,
        index,
        address,
        exists: false,
        dataStatus: "closed",
        label: null,
        lamports: 0,
        account: null,
        rawAccount: null,
      };
    }
    if (info.owner.equals(this.instructions.programId) && info.data.length === WALLET_LENGTH) {
      const wallet = parseWallet(Buffer.from(info.data));
      return {
        agentAsset,
        index,
        address,
        exists: true,
        dataStatus: wallet.isActive ? "active" : wallet.isRecoveryOnly ? "recovery" : "invalid",
        label: wallet.label,
        lamports: info.lamports,
        account: wallet,
        rawAccount: info,
      };
    }
    const dusted = info.owner.equals(SystemProgram.programId) && info.data.length === 0;
    return {
      agentAsset,
      index,
      address,
      exists: false,
      dataStatus: dusted ? "dusted" : "invalid",
      label: null,
      lamports: info.lamports,
      account: null,
      rawAccount: info,
    };
  }
}
