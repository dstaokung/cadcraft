#!/bin/bash
# Prepares a Claude Code cloud session so `cargo xtask ci` runs without a cold build:
# installs the wasm target and pre-compiles the workspace (tests, clippy, wasm check)
# into target/, which the cloud container caches after this hook finishes.
set -euo pipefail

if [ "${CLAUDE_CODE_REMOTE:-}" != "true" ]; then
  exit 0
fi

cd "$CLAUDE_PROJECT_DIR"

# plan/ is gitignored and local-only. When the session also has the private
# cadcraft-plan repository checked out next to this one, link it in as plan/.
PLAN_REPO="$(dirname "$CLAUDE_PROJECT_DIR")/cadcraft-plan"
if [ -d "$PLAN_REPO" ] && [ ! -e plan ]; then
  ln -s "$PLAN_REPO" plan
  # .gitignore's /plan/ matches only directories, so hide the symlink too.
  grep -qxF '/plan' .git/info/exclude 2>/dev/null || echo '/plan' >> .git/info/exclude
fi

rustup component add rustfmt clippy
rustup target add wasm32-unknown-unknown

cargo fetch --locked

# Warm the caches the quality gates use; failures here must not block the session.
cargo test --workspace --no-run --locked || echo "session-start: test pre-build failed" >&2
cargo clippy --workspace --all-targets --locked || echo "session-start: clippy pre-build failed" >&2
cargo xtask wasm || echo "session-start: wasm pre-check failed" >&2
