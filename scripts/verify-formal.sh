#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

NO_DNA=1 cargo kani \
  --manifest-path programs/agent-vault/Cargo.toml \
  --features no-entrypoint \
  --no-default-features \
  --default-unwind 40 \
  --output-format terse \
  --fail-fast
