#!/usr/bin/env bash
# Verify that Python / npm packaging versions track the Rust crate version.
# Called from packaging CI and optionally from local release checks.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

crate_version="$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)"
py_crate_version="$(sed -n 's/^version = "\(.*\)"/\1/p' djvu-py/Cargo.toml | head -1)"
manifest_version="$(python3 - <<'PY'
import json
print(json.load(open(".release-please-manifest.json"))["."])
PY
)"

fail=0
if [[ -z "$crate_version" ]]; then
  echo "::error::Could not read version from Cargo.toml" >&2
  exit 1
fi

if [[ "$py_crate_version" != "$crate_version" ]]; then
  echo "::error::djvu-py/Cargo.toml version ($py_crate_version) != crate version ($crate_version)" >&2
  fail=1
fi

if [[ "$manifest_version" != "$crate_version" ]]; then
  echo "::error::.release-please-manifest.json ($manifest_version) != crate version ($crate_version)" >&2
  fail=1
fi

if ! grep -q 'dynamic = \["version"\]' djvu-py/pyproject.toml; then
  echo "::error::djvu-py/pyproject.toml must use dynamic = [\"version\"] (from Cargo.toml)" >&2
  fail=1
fi

if [[ "${CHECK_NPM_PACKAGE:-}" == "1" ]]; then
  npm_pkg="${NPM_PACKAGE_JSON:-examples/wasm/pkg/package.json}"
  if [[ ! -f "$npm_pkg" ]]; then
    echo "::error::npm package.json not found at $npm_pkg (run make wasm first)" >&2
    fail=1
  else
    npm_version="$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["version"])' "$npm_pkg")"
    if [[ "$npm_version" != "$crate_version" ]]; then
      echo "::error::npm package version ($npm_version) != crate version ($crate_version)" >&2
      fail=1
    fi
  fi
fi

if [[ "$fail" -ne 0 ]]; then
  exit 1
fi

echo "package versions OK: crate=$crate_version py=$py_crate_version manifest=$manifest_version"
