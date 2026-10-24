#!/bin/bash
# Prepares a Claude Code cloud session so `cargo xtask ci` runs without a cold build:
# installs the wasm target and pre-compiles the workspace (tests, clippy, wasm check)
# into target/, which the cloud container caches after this hook finishes.
set -euo pipefail

if [ "${CLAUDE_CODE_REMOTE:-}" != "true" ]; then
  exit 0
fi

cd "$CLAUDE_PROJECT_DIR"

rustup component add rustfmt clippy
rustup target add wasm32-unknown-unknown

cargo fetch --locked

# Warm the caches the quality gates use; failures here must not block the session.
cargo test --workspace --no-run --locked || echo "session-start: test pre-build failed" >&2
cargo clippy --workspace --all-targets --locked || echo "session-start: clippy pre-build failed" >&2
cargo xtask wasm || echo "session-start: wasm pre-check failed" >&2
