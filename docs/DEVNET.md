# Agent Vault Devnet

Agent Vault currently has a devnet version. It is unaudited and not a mainnet
release.

## Fast Path

Program checks:

```bash
NO_DNA=1 cargo test --offline
NO_DNA=1 cargo test --offline --manifest-path tests/runtime/Cargo.toml -- --test-threads=1
```

SDK preflight:

```bash
cd ../agent-vault-sdk
NO_DNA=1 npm run check
NO_DNA=1 npm run e2e:devnet
```

`npm run e2e:devnet` is preflight-only unless `AGENT_VAULT_E2E_SEND=1` is set.

## Deployment

```text
Program ID:         36u7KMBuxjExvU6V2nfTX5SnNdYMGUupFiYouLzrgpfW
8004 registry:     8oo4J9tBB3Hna1jRQ3rWvJjojqM5DYTDJo5cejUuJy3C
8004 collection:   6CTyGPcn8dMwKEqgtvx2XCpkGUd7uqCVK6937RSM5bhA
Release status:    deployed
```

The release manifest is tracked in
[`docs/RELEASE_MANIFEST.devnet.json`](./RELEASE_MANIFEST.devnet.json). Canonical
deployment verification is RPC-backed in the SDK devnet preflight: it checks the
program account, ProgramData account, deployed ELF hash and size, ProgramData
address, ProgramData upgrade authority, global config PDA, global config bump,
and expected global config fields against that manifest before signed SDK writes.

## Verify Locally

From the program repository:

```bash
NO_DNA=1 cargo test --offline
NO_DNA=1 cargo test --offline --manifest-path tests/runtime/Cargo.toml -- --test-threads=1
NO_DNA=1 cargo test --offline --manifest-path tests/runtime/Cargo.toml devnet_release_cost_report -- --nocapture --test-threads=1
NO_DNA=1 ./scripts/verify-formal.sh
```

Full local release verification:

```bash
NO_DNA=1 ./scripts/verify-devnet-release.sh
```

That script runs formatting, Clippy, unit tests, SBF build checks, LiteSVM
runtime tests, localnet e2e, Kani harnesses, and local release artifact hash/size
checks. It does not perform live RPC reads; use the SDK preflight below for
onchain devnet deployment verification.

## SDK Preflight

From the SDK repository:

```bash
NO_DNA=1 npm ci
NO_DNA=1 npm run check
NO_DNA=1 npm run e2e:devnet
```

`npm run e2e:devnet` defaults to preflight-only mode. It verifies the live devnet
deployment and exits before any write. Set `AGENT_VAULT_E2E_SEND=1` only when
the signer is funded and you intentionally want to run the live write flow.

## Flow Coverage

The program repo covers the full V0 program instruction surface through LiteSVM
runtime tests and `scripts/localnet-e2e.py`. The separate Agent-Vault-SDK repo
provides devnet preflight against the live deployment and gated send-mode
coverage for SDK transaction construction. `initialize_global_config` is sent
by SDK devnet e2e only when `AGENT_VAULT_INIT_GLOBAL=1` creates a missing
global config; otherwise the e2e counts it as deployment-verified by the live
global-config preflight:

```text
initialize_global_config
init_vault_config
create_wallet
update_wallet_label
deposit_sol
withdraw_sol
transfer_sol
close_wallet
reopen_wallet_for_recovery
create_wallet_ata
transfer_spl
wrap_sol
unwrap_sol
close_wallet_ata
execute_cpi_checked
```

The runtime `devnet_release_cost_report` test prints compute units, rent
snapshots, and release cost categories for the deterministic LiteSVM fixtures.

## Mainnet

There is no mainnet release. Mainnet writes must remain blocked until a
canonical mainnet manifest and upgrade policy are published and implemented in
the SDK verification path.
