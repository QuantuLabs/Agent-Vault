#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

NO_DNA=1 cargo fmt --check
NO_DNA=1 cargo clippy --offline --all-targets -- -D warnings
NO_DNA=1 cargo test --offline

BUILD_LOG="${TMPDIR:-/tmp}/agent-vault-build-sbf.log"
NO_DNA=1 cargo build-sbf 2>&1 | tee "$BUILD_LOG"

if grep -E "Stack offset|overwrites values in the frame" "$BUILD_LOG" >/dev/null; then
  echo "SBF stack verification failed" >&2
  exit 1
fi

NO_DNA=1 cargo test --offline --manifest-path tests/runtime/Cargo.toml -- --test-threads=1
NO_DNA=1 cargo test --offline --manifest-path tests/runtime/Cargo.toml devnet_release_cost_report -- --nocapture --test-threads=1
NO_DNA=1 ./scripts/verify-formal.sh

node <<'NODE'
const crypto = require('crypto');
const fs = require('fs');

const manifest = JSON.parse(fs.readFileSync('docs/RELEASE_MANIFEST.devnet.json', 'utf8'));
const elf = fs.readFileSync('target/deploy/agent_vault.so');
const actualHash = crypto.createHash('sha256').update(elf).digest('hex');
const expectedHash = manifest.program.sbfElfSha256;

if (actualHash !== expectedHash) {
  console.error(`SBF hash mismatch: expected ${expectedHash}, got ${actualHash}`);
  process.exit(1);
}

const expectedSize = manifest.program.sbfElfSizeBytes;
if (elf.length !== expectedSize) {
  console.error(`SBF size mismatch: expected ${expectedSize}, got ${elf.length}`);
  process.exit(1);
}

console.log(`devnet release artifact verified: ${actualHash}`);
NODE
