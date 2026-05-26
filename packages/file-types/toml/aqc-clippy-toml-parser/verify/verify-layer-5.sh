#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."

echo "=== Layer 5: Compilation ==="

echo "Building workspace..."
cargo build --workspace --manifest-path Cargo.toml

echo "Running generator (uses cached conf.rs if present)..."
cargo run -p aqc-clippy-toml-parser-generator --bin generate

echo "PASS: Layer 5"
