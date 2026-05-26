#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."

echo "=== Layer 1: Tree Structure ==="

required=(
  "Cargo.toml"
  "manifest.toml"
  "crates/types/Cargo.toml"
  "crates/types/src/lib.rs"
  "crates/types/src/clippy_toml.rs"
  "crates/runtime/Cargo.toml"
  "crates/runtime/src/lib.rs"
  "crates/generator/Cargo.toml"
  "crates/generator/src/main.rs"
)

for path in "${required[@]}"; do
  if [[ ! -f "$path" ]]; then
    echo "FAIL: missing $path"
    exit 1
  fi
  echo "ok  $path"
done

echo "PASS: Layer 1"
