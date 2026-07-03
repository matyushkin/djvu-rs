# Local developer gates mirroring CI. `make hooks` once to enable the pre-push hook.
.PHONY: check hooks pgo

## Run the same deterministic gates CI enforces (fmt, clippy, no_std, wasm, tests).
check:
	@scripts/check.sh

## Enable the version-controlled git hooks (pre-push runs `make check`).
hooks:
	@git config core.hooksPath .githooks && echo "git hooks enabled: core.hooksPath=.githooks"

## Opt-in profile-guided optimization build (layers PGO over fat-LTO). Trains on
## the corpus, then PGO-rebuilds the `djvu` CLI. ~15% faster cold render; see the
## "PGO" entry in PERF_EXPERIMENTS.md. Needs the llvm-tools-preview component.
pgo:
	@scripts/pgo.sh
