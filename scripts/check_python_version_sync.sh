#!/usr/bin/env bash
# Fail if djvu-py PEP 621 version drifts from the Rust crate versions (#692).
set -euo pipefail
cd "$(git rev-parse --show-toplevel)"
root=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)
py_cargo=$(sed -n 's/^version = "\(.*\)"/\1/p' djvu-py/Cargo.toml | head -1)
pyproject=$(sed -n 's/^version = "\(.*\)"/\1/p' djvu-py/pyproject.toml | head -1)
if [[ -z "$root" || -z "$py_cargo" || -z "$pyproject" ]]; then
  echo "!! failed to parse versions" >&2
  exit 1
fi
if [[ "$root" != "$py_cargo" || "$root" != "$pyproject" ]]; then
  echo "!! version mismatch: Cargo.toml=$root djvu-py/Cargo.toml=$py_cargo pyproject.toml=$pyproject" >&2
  exit 1
fi
echo "python package version sync ok ($root)"
