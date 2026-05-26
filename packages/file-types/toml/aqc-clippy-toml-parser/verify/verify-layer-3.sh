#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."

echo "=== Layer 3: Generator Contract ==="

generator_file="crates/generator/src/main.rs"
types_file="crates/types/src/clippy_toml.rs"
manifest="manifest.toml"

pinned_tag=$(grep -E '^clippy_tag = ' "$manifest" | sed -E 's/.*"([^"]+)".*/\1/')
if [[ -z "${pinned_tag}" ]]; then
  echo "FAIL: clippy_tag not declared in manifest.toml"
  exit 1
fi
echo "ok  manifest declares clippy_tag = $pinned_tag"

if ! grep -qF "CLIPPY_TAG: &str = \"${pinned_tag}\"" "$generator_file"; then
  echo "FAIL: generator CLIPPY_TAG const does not match manifest pin ($pinned_tag)"
  exit 1
fi
echo "ok  generator CLIPPY_TAG matches manifest pin"

if ! grep -qF "rust-lang/rust-clippy" "$generator_file"; then
  echo "FAIL: generator does not reference rust-clippy repo"
  exit 1
fi
echo "ok  generator references rust-clippy"

if ! grep -qF "define_Conf" "$generator_file"; then
  echo "FAIL: generator does not parse define_Conf! macro"
  exit 1
fi
echo "ok  generator parses define_Conf!"

if ! grep -qF "// Source: rust-lang/rust-clippy tag ${pinned_tag}" "$types_file"; then
  echo "FAIL: generated file does not carry source-tag header for $pinned_tag"
  exit 1
fi
echo "ok  generated file carries source-tag header"

if ! grep -q 'rename_all.*kebab-case' "$types_file"; then
  echo "FAIL: generated types do not use kebab-case"
  exit 1
fi
echo "ok  generated types use kebab-case"

echo "PASS: Layer 3"
