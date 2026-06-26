# Local developer gates mirroring CI. `make hooks` once to enable the pre-push hook.
.PHONY: check hooks

## Run the same deterministic gates CI enforces (fmt, clippy, no_std, wasm, tests).
check:
	@scripts/check.sh

## Enable the version-controlled git hooks (pre-push runs `make check`).
hooks:
	@git config core.hooksPath .githooks && echo "git hooks enabled: core.hooksPath=.githooks"
