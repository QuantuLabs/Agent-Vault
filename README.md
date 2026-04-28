# Agent Vault

Agent Vault is a Pinocchio program on Solana that lets the live owner of an
8004 Metaplex Core Asset control multiple indexed PDA wallets.

## Status

This repository is a **work in progress**.

- The program is unaudited.
- There is no mainnet release.
- The devnet artifact is a local candidate and is not deployed yet.
- Do not use this with valuable assets.

## Current Deployment Facts

```text
Program ID:        36u7KMBuxjExvU6V2nfTX5SnNdYMGUupFiYouLzrgpfW
Devnet registry:  8oo4J9tBB3Hna1jRQ3rWvJjojqM5DYTDJo5cejUuJy3C
Devnet collection: 6CTyGPcn8dMwKEqgtvx2XCpkGUd7uqCVK6937RSM5bhA
Devnet status:    candidate-not-deployed
```

The devnet candidate release metadata is tracked in
[`docs/RELEASE_MANIFEST.devnet.json`](docs/RELEASE_MANIFEST.devnet.json).

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
writable wallet-controlled token accounts.

Mainnet clients must fail closed unless the canonical deployment, global config,
ProgramData hash, and upgrade authority policy all verify against a published
release manifest.

## Release Metadata

- [Devnet release manifest](docs/RELEASE_MANIFEST.devnet.json)

## Build And Test

```bash
NO_DNA=1 cargo test --offline
NO_DNA=1 cargo build-sbf
NO_DNA=1 cargo test --offline --manifest-path tests/runtime/Cargo.toml
NO_DNA=1 ./scripts/verify-formal.sh
```

Full local release verification:

```bash
NO_DNA=1 ./scripts/verify-devnet-release.sh
```

The verification script runs formatting, unit tests, SBF build, SBF stack-log
checks, LiteSVM runtime tests, Kani harnesses, and release artifact hash/size
checks.

## Repository Layout

```text
programs/agent-vault   Pinocchio onchain program
programs/mock-amm      test-only mock target used by checked-CPI runtime tests
tests/runtime          LiteSVM runtime tests
scripts                release verification helpers
```

## SDK

The intended public TypeScript SDK surface is `8004-solana`, grouped under a
`.wallets` namespace. SDK integration is not implemented in this repository yet.

## License

Apache-2.0. Copyright 2026 Quantu Labs.
