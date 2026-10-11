#!/bin/bash
# SessionStart hook for Claude Code cloud sessions: make the local gates from
# CLAUDE.md runnable out of the box — `make check` (scripts/check.sh) and
# `make -C djvu-py py-test` (the required `djvu-py (pytest)` check). The
# container state is cached after this hook completes, so the warm-up builds
# below are paid once.
set -euo pipefail

if [ "${CLAUDE_CODE_REMOTE:-}" != "true" ]; then
  exit 0
fi

cd "$CLAUDE_PROJECT_DIR"

# wasm32 gates in scripts/check.sh exit 1 without this target (#448 class).
rustup target add wasm32-unknown-unknown

# Version-controlled git hooks: commit-msg (no AI co-author trailers),
# pre-commit (fmt/clippy/no_std) and pre-push (scripts/check.sh).
git config core.hooksPath .githooks

# djvu-py/Makefile builds its venv with uv.
if ! command -v uv >/dev/null 2>&1; then
  python3 -m pip install --quiet --user uv
  echo "export PATH=\"\$HOME/.local/bin:\$PATH\"" >> "${CLAUDE_ENV_FILE:-/dev/null}"
  export PATH="$HOME/.local/bin:$PATH"
fi

cargo fetch

# Warm the target dir with the same feature sets check.sh / CI test.
cargo test --workspace --exclude djvu-py --features cli,tiff,async --no-run
cargo clippy --all-targets -- -D warnings

# Python bindings: venv + release build of the extension module.
make -C djvu-py py-build
