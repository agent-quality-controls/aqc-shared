# Rustfmt TOML Engine

## Summary

Added `aqc-rustfmt-toml-engine` as a shared AQC TOML engine for `rustfmt.toml`.
The engine models Rustfmt scalar and list settings with shared core requirement
primitives, resolves policy contributions with provenance, and reconciles
current TOML into expected bytes and findings.

## Decisions Made

- Rustfmt settings are typed as `RustfmtScalarSetting` and
  `RustfmtListSetting`, with file-native snake_case mapping through
  `file_key()`.
- Scalar assertions use `ConfigScalar` plus `Equals`, `OneOf`, `Present`, and
  `Absent`.
- List settings use core `ListRequirements` and `ResolvedListRequirements`.
- Malformed list values are reported and normalized before reconcile writes.
- File-tree rules are not in this engine; runner/topology owns placement.

Alternatives rejected:

- Stringly typed Rustfmt setting names, because policies should not guess file
  keys.
- Engine-level filesystem scanning, because the engine owns one file's bytes,
  not workspace topology.

## Key Files For Context

- `Cargo.toml`
- `packages/file-types/toml/aqc-rustfmt-toml-engine/src/requirement.rs`
- `packages/file-types/toml/aqc-rustfmt-toml-engine/src/engine.rs`
- `packages/file-types/toml/aqc-rustfmt-toml-engine/src/reconcile/settings.rs`
- `packages/file-types/toml/aqc-rustfmt-toml-engine/tests/reconcile.rs`

## Verification

- `cargo fmt --all --check`
- `cargo test -p aqc-rustfmt-toml-engine`
- `cargo publish --dry-run --allow-dirty`
- Specular verification from the parent `agent-quality-controls` workspace via
  the Guardrails Rustfmt vertical spec.

## Next Steps

- Publish `aqc-rustfmt-toml-engine` before publishing
  `g3rs-rustfmt-adapter`, because the adapter depends on this crate by version.
