# Agent Vault

Agent Vault is a Pinocchio program on Solana that lets the live owner of an
8004 Metaplex Core Asset control multiple indexed PDA wallets.

## Start Here

For application integration, start with the
[Agent Vault SDK README](https://github.com/QuantuLabs/Agent-Vault-SDK#readme).
The app-facing flow is:

```ts
const registered = await identity.registerAgent(metadataUri, { collectionPointer })
const agentAsset = registered.asset
const vault = AgentVaultClient.devnet({ connection, signer })
const agent = vault.agent(agentAsset)

await agent.wallets.setup({ labels: ["treasury", "trading"] })
const wallets = await agent.wallets.listAll()
```

`agentAsset` is the 8004 Core Asset pubkey. `wallet` is a numeric index inside
that agent vault (`0`, `1`, ...), not the wallet PDA address.

For program work, build and test this repo with:

```bash
NO_DNA=1 cargo test --offline
NO_DNA=1 cargo test --offline --manifest-path tests/runtime/Cargo.toml -- --test-threads=1
```

## Status

This is the current devnet version.

- The program is unaudited.
- There is no mainnet release.
- Do not use this with valuable assets.

## Current Deployment Facts

```text
Program ID:        36u7KMBuxjExvU6V2nfTX5SnNdYMGUupFiYouLzrgpfW
Devnet registry:  8oo4J9tBB3Hna1jRQ3rWvJjojqM5DYTDJo5cejUuJy3C
Devnet collection: 6CTyGPcn8dMwKEqgtvx2XCpkGUd7uqCVK6937RSM5bhA
Devnet status:    deployed
```

The devnet release metadata is tracked in
[`docs/RELEASE_MANIFEST.devnet.json`](docs/RELEASE_MANIFEST.devnet.json).
The `Devnet collection` above is the onchain Core collection enforced by Agent
Vault, not the 8004 `collectionPointer` string passed to the SDK.

## What It Does

One 8004 Core Asset controls an indexed family of PDA wallets:

```text
agent_asset
  - wallet #0  PDA(["agent_vault", agent_asset, 0u16_le])
  - wallet #1  PDA(["agent_vault", agent_asset, 1u16_le])
  - wallet #2  PDA(["agent_vault", agent_asset, 2u16_le])
```

The live Core Asset owner can create wallets, withdraw SOL, transfer SOL, manage
wallet ATAs, and authorize checked CPI execution for DeFi flows. SOL deposits are
permissionless.

## V0 Scope

- Indexed PDA wallets per 8004 Core Asset.
- SOL deposits, withdrawals, transfers, wallet close, and rent recovery.
- SPL Token transfers through wallet-owned ATAs.
- Minimal Token-2022 support, including checked transfer-fee paths.
- WSOL wrap and unwrap.
- `execute_cpi_checked` for DeFi/swap composition with explicit post-checks.
- Immutable global config for the canonical deployment constants.

V0 intentionally does not include delegation, spending limits, allowlists, or
mainnet upgrade governance. Those belong in later versioned policy accounts.

## Security Model

Protected instructions require the signer to be the live owner encoded in the
Metaplex Core Asset. Vault activation also validates the 8004 `AgentAccount`
against the expected registry and collection.

`execute_cpi_checked` is powerful because the wallet PDA signs a target CPI. The
instruction therefore requires explicit post-checks, rejects direct Token/ATA and
loader targets, keeps the wallet account readonly, and enforces custody checks for
writable wallet-controlled token accounts. SPL Token multisig authorities that
are satisfiable by the wallet PDA are intentionally rejected in V0.

The intended SDK/mainnet client behavior is fail-closed unless the canonical
deployment, global config, ProgramData hash, and upgrade authority policy all
verify against a published release manifest. The TypeScript SDK implements this
verification in the separate `agent-vault-sdk` repository.

## Docs Map

- App integration: [Agent Vault SDK README](https://github.com/QuantuLabs/Agent-Vault-SDK#readme)
- Program architecture: [`docs/PROGRAM.md`](docs/PROGRAM.md)
- Devnet verification: [`docs/DEVNET.md`](docs/DEVNET.md)
- Devnet release manifest: [`docs/RELEASE_MANIFEST.devnet.json`](docs/RELEASE_MANIFEST.devnet.json)
- Security policy: [`SECURITY.md`](SECURITY.md)

## Build And Test

```bash
NO_DNA=1 cargo clippy --offline --all-targets -- -D warnings
NO_DNA=1 cargo test --offline
NO_DNA=1 cargo build-sbf
NO_DNA=1 cargo test --offline --manifest-path tests/runtime/Cargo.toml -- --test-threads=1
NO_DNA=1 scripts/localnet-e2e.py
NO_DNA=1 ./scripts/verify-formal.sh
```

Full local release verification:

```bash
NO_DNA=1 ./scripts/verify-devnet-release.sh
```

The verification script runs formatting, Clippy with warnings denied, unit
tests, SBF build, SBF stack-log checks, LiteSVM runtime tests, a localnet e2e
validator run, Kani harnesses, and release artifact hash/size checks.

The localnet e2e script requires `solana-test-validator` and Python `solders`.

## Repository Layout

```text
programs/agent-vault   Pinocchio onchain program
programs/mock-amm      test-only mock target used by checked-CPI runtime tests
tests/runtime          LiteSVM runtime tests
scripts                release verification helpers
```

## SDK

The TypeScript SDK is maintained in the separate
[Agent-Vault-SDK](https://github.com/QuantuLabs/Agent-Vault-SDK) repository. Its
npm package name is `agent-vault`; it provides the high-level `.identities` and
`.wallets` developer surface for devnet testing.

```ts
const agent = vault.agent(agentAsset)
const wallets = await agent.wallets.listAll()
```

`agentAsset` is the 8004 Core Asset public key for the agent. The `wallet`
parameter used by write methods is the numeric wallet index inside that agent
vault (`0`, `1`, ...); the SDK derives the wallet PDA address from
`agentAsset + wallet`.

## License

Apache-2.0. Copyright 2026 Quantu Labs.
