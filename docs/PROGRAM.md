# Agent Vault Program

Agent Vault is a no-Anchor Pinocchio program for 8004 Metaplex Core Asset
owners. One live Core Asset owner controls an indexed set of PDA wallets derived
from the asset address.

## SDK Builder Notes

- `agent_asset` is passed to the program as the Metaplex Core Asset account.
- The SDK name for that pubkey is `agentAsset`.
- Wallet instructions take a `u16` index; the SDK option is usually named `wallet`, `from`, or `to`.
- The wallet PDA address is always derived from `agentAsset + index`; users should not type it into write APIs.
- Listing wallets is a read-side SDK operation, not a program instruction.

## PDA Model

```text
global_config = PDA(["global_config"])
vault_config  = PDA(["vault_config", agent_asset])
agent_wallet  = PDA(["agent_vault", agent_asset, index_u16_le])
agent_account = PDA(["agent", agent_asset], 8004 registry program)
```

`vault_config.wallet_count` is the onchain source of truth for wallet
enumeration. Clients derive wallet indexes from `0..wallet_count` and fetch the
PDA accounts directly; no indexer is required for wallet listing. The SDK
exposes this as paginated `list(...)` and explicit full enumeration through
`listAll(...)`.

## Accounts

```text
AgentVaultGlobalConfig  160 bytes
AgentVaultConfig         24 bytes
AgentWallet              32 bytes
```

Each account uses a fixed discriminator, version byte, stored bump, and reserved
bytes for forward-compatible policies. V0 does not resize these accounts to add
future features.

## Instruction Groups

```text
0..31   core wallet lifecycle and SOL
32..63  SPL Token, Token-2022, and WSOL helpers
64..95  checked CPI / DeFi composition
96..127 future policies
```

V0 instructions:

```text
0   initialize_global_config
1   init_vault_config
2   create_wallet
3   update_wallet_label
4   deposit_sol
5   withdraw_sol
6   transfer_sol
7   close_wallet
8   reopen_wallet_for_recovery
32  create_wallet_ata
33  transfer_spl
34  wrap_sol
35  unwrap_sol
36  close_wallet_ata
64  execute_cpi_checked
```

## Ownership Checks

Protected instructions require the signer to be the live owner stored in the
Metaplex Core Asset. Vault initialization also validates the 8004 registry
`AgentAccount` PDA for the same asset and expected collection.

Hot SOL paths avoid loading unused config accounts where possible:

```text
deposit_sol    permissionless, wallet-only validation
withdraw_sol   live Core owner + compiled collection + wallet PDA
transfer_sol   live Core owner + compiled collection + both wallet PDAs
```

Token, WSOL, recovery, and checked-CPI paths validate `global_config` and
`vault_config` before using their semantic fields.

## Token Scope

Tokenkeg support covers wallet-owned ATA creation, checked transfers, and ATA
close.

Token-2022 support is intentionally minimal:

```text
mint extensions:    none, TransferFeeConfig
account extensions: none, ImmutableOwner, TransferFeeAmount
```

Unsupported Token-2022 extensions are rejected by typed token helpers. Generic
checked CPI stays protocol-neutral and relies on target denylist plus explicit
post-checks.

## Checked CPI

`execute_cpi_checked` lets the wallet PDA sign one target CPI while enforcing
postconditions after the call. The wallet is inserted as a readonly signer at
`wallet_meta_index`; the program never invents signer authority for other
accounts.

The instruction rejects:

```text
Agent Vault itself
Solana loader programs
Tokenkeg
Token-2022
Associated Token Program
non-executable targets
writable wallet account
protected account duplication
unchecked writable wallet-controlled token custody
wallet-controlled SPL Token multisig custody
unchecked writable non-token state
```

At least one economic post-check is required. Writable wallet-controlled token
accounts require both an economic check and a custody-integrity check. Writable
non-token accounts require `AccountStateEquals`. Frozen, malformed, or SPL
multisig-controlled wallet token custody is rejected in V0.

## Recovery

`close_wallet` can close a wallet whose SOL balance is at the rent threshold.
Wallet indexes are never reused. If supported wallet-owned ATAs or WSOL are
stranded after close, `reopen_wallet_for_recovery` recreates the same PDA
authority in recovery-only mode so the holder can clean them up through typed V0
helpers.

Recovery-only wallets cannot be used for deposits, wrapping, checked CPI, or as
normal active wallets.

## Verification

Primary local verification:

```bash
NO_DNA=1 cargo test --offline
NO_DNA=1 cargo test --offline --manifest-path tests/runtime/Cargo.toml -- --test-threads=1
NO_DNA=1 ./scripts/verify-formal.sh
```

Release verification:

```bash
NO_DNA=1 ./scripts/verify-devnet-release.sh
```

The runtime tests cover wallet lifecycle, SOL flows, Tokenkeg, Token-2022,
WSOL, checked CPI, recovery, rent snapshots, CU gates, and the spec coverage
matrix. Kani harnesses cover parser, layout, PDA, checked-CPI planning, label,
fee math, and lamport arithmetic invariants.
