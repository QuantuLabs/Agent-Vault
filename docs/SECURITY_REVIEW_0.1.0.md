# Agent Vault v0.1.0 Internal Security Review

Date: 2026-05-14

Scope:

- `programs/agent-vault` Pinocchio program
- `programs/mock-amm` test target
- LiteSVM runtime tests and release scripts
- README, security policy, and app-integration docs

This is an internal engineering review, not an external production audit.
Mainnet remains blocked until a canonical mainnet manifest, upgrade policy, and
external review are complete.

## Result

No critical or high-severity open issue remains in the reviewed V0 scope.

Fixed during review:

- `execute_cpi_checked` no longer accepts a `TokenCustodyEquals` post-check
  that moves a wallet ATA's final authority, delegate, or close authority out
  of wallet-safe custody.
- Fixed-account instructions now reject extra accounts instead of silently
  ignoring them. This reduces account-surface ambiguity for clients and tests.
- Host-side PDA derivation now allows the Solana maximum of 16 seeds, matching
  runtime semantics for local tests and formal harnesses.
- Added read-only live devnet verification for Program, ProgramData hash, byte
  size, upgrade authority, and global config manifest fields.
- Release verification now asserts manifest build metadata as well as artifact
  hash and size.
- Stale SDK docs were corrected to use `@quantulabs/agent-vault` and to avoid
  the removed high-level `.fund` flow.

Deployment note: these source-level hardenings require a new SBF deployment and
release-manifest update before they are part of the live devnet program at
`36u7KMBuxjExvU6V2nfTX5SnNdYMGUupFiYouLzrgpfW`.

Local reviewed SBF artifact:

```text
target/deploy/agent_vault.so
size:   151096 bytes
sha256: e4fa6135faa095070851344fe034089780ce0976cd2d856ca16a319a5b53b5a3
```

## Reviewed Invariants

- Global config is a canonical PDA, immutable after init, and pinned to devnet
  registry, collection, fee treasury, and activation fee constants.
- Vault init requires the live Metaplex Core Asset owner and the 8004
  `AgentAccount` PDA for the same asset and collection.
- Wallets are indexed PDAs derived from `agent_asset + u16 index`; cross-agent
  wallet substitution is rejected.
- SOL withdraw, transfer, close, WSOL wrap, and checked CPI preserve the wallet
  rent floor where required.
- Tokenkeg and supported Token-2022 paths validate owner, mint, decimals, ATA
  derivation, authority, close authority, delegate state, and unsupported
  extensions.
- `execute_cpi_checked` keeps the wallet account readonly, rejects direct
  Token/ATA/loader targets, requires economic post-checks, and requires custody
  or account-state checks for writable accounts.
- Recovery-only wallets can clean stranded supported assets but cannot be used
  for hot paths such as deposit, wrap, or checked CPI.

## Residual Risk

- `execute_cpi_checked` is intentionally powerful. Route helpers must generate
  strict post-checks for min output, max input, custody, and writable account
  state; weak post-checks can authorize bad trades.
- Token-2022 support is minimal by design. Unsupported extensions are rejected
  and need explicit review before support is expanded.
- Devnet upgrade authority still exists under the devnet policy. Production
  mainnet needs a published manifest and non-bypassable governance or revoked
  authority.
- This review does not replace an independent external audit.

## Verification

Commands run during this review:

```bash
NO_DNA=1 cargo fmt --check
NO_DNA=1 cargo fmt --check --manifest-path tests/runtime/Cargo.toml
NO_DNA=1 cargo clippy --offline --all-targets -- -D warnings
NO_DNA=1 cargo clippy --offline --manifest-path tests/runtime/Cargo.toml --all-targets -- -D warnings
NO_DNA=1 cargo test --offline
NO_DNA=1 cargo build-sbf
NO_DNA=1 cargo test --offline --manifest-path tests/runtime/Cargo.toml -- --test-threads=1
NO_DNA=1 cargo test --offline --manifest-path tests/runtime/Cargo.toml fixed_account_instructions_reject_extra_accounts -- --test-threads=1
NO_DNA=1 cargo test --offline --manifest-path tests/runtime/Cargo.toml execute_cpi_checked_rejects_postchecked_wallet_ata_custody_loss -- --test-threads=1
NO_DNA=1 cargo test --offline --manifest-path tests/runtime/Cargo.toml execute_cpi_checked_token_custody_equals_supports_new_wallet_control -- --test-threads=1
NO_DNA=1 ./scripts/verify-devnet-onchain.sh
NO_DNA=1 ./scripts/verify-formal.sh
NO_DNA=1 scripts/localnet-e2e.py
```

`scripts/verify-devnet-release.sh` intentionally remains tied to the published
devnet manifest. It should be rerun after deployment and manifest update, once
the new SBF hash is the live devnet hash.

Full-suite release verification before publishing a manifest:

```bash
NO_DNA=1 cargo clippy --offline --all-targets -- -D warnings
NO_DNA=1 cargo test --offline --manifest-path tests/runtime/Cargo.toml -- --test-threads=1
NO_DNA=1 ./scripts/verify-devnet-onchain.sh
NO_DNA=1 ./scripts/verify-formal.sh
NO_DNA=1 ./scripts/verify-devnet-release.sh
```
