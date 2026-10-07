#!/bin/bash
# SessionStart hook for Claude Code cloud sessions: make `make check`
# (scripts/check.sh) runnable out of the box. The container state is cached
# after this hook completes, so the warm-up builds below are paid once.
set -euo pipefail

if [ "${CLAUDE_CODE_REMOTE:-}" != "true" ]; then
  exit 0
fi

cd "$CLAUDE_PROJECT_DIR"

# wasm32 gates in scripts/check.sh exit 1 without this target (#448 class).
rustup target add wasm32-unknown-unknown

# Version-controlled git hooks (commit-msg attribution rule, pre-push gates).
git config core.hooksPath .githooks

cargo fetch

# Warm the target dir with the same feature sets check.sh / CI test.
cargo test --workspace --exclude djvu-py --features cli,tiff,async --no-run
cargo clippy --all-targets -- -D warnings
