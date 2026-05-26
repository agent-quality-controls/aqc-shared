#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."

echo "=== Layer 2: Public API ==="

types_file="crates/types/src/clippy_toml.rs"

required_decls=(
  "pub struct ClippyToml"
  "pub enum DisallowedPath"
  "pub enum MatchLintBehaviour"
  "pub enum PubUnderscoreFieldsBehaviour"
  "pub enum InherentImplLintScope"
  "pub struct SourceItemOrdering"
  "pub enum SourceItemOrderingCategory"
  "pub struct SourceItemOrderingModuleItemGroupings"
  "pub enum SourceItemOrderingModuleItemKind"
  "pub struct SourceItemOrderingTraitAssocItemKinds"
  "pub enum SourceItemOrderingTraitAssocItemKind"
  "pub enum SourceItemOrderingWithinModuleItemGroupings"
  "impl Default for ClippyToml"
)

for decl in "${required_decls[@]}"; do
  if ! grep -qF "$decl" "$types_file"; then
    echo "FAIL: missing declaration: $decl"
    exit 1
  fi
  echo "ok  $decl"
done

if ! grep -q '#\[derive.*Serialize.*Deserialize' "$types_file"; then
  echo "FAIL: missing serde derives"
  exit 1
fi
echo "ok  serde derives present"

echo "PASS: Layer 2"
