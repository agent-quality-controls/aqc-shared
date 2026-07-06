# Rust Toolchain Engine Redesign

## Summary

Rebuilt `aqc-rust-toolchain-toml-engine` as a fixed-field engine for `rust-toolchain.toml`.
The engine now exposes typed channel, profile, and path values, uses core scalar/list requirements, and rejects invalid rustup file shapes before writing.

## Decisions Made

- Replaced generic scalar/list setting maps with fixed public fields: `channel`, `path`, `profile`, `components`, `targets`, and `closed_settings`.
- Kept `path` mutually exclusive with `channel`, `profile`, `components`, and `targets`, because rustup treats path toolchains as a different file shape.
- Made invalid file shapes hard failures and missing required fields repairable mismatches.
- Added serde support to `DottedVersion` so downstream policies can parse and emit MSRV values through typed JSON params.
- Rejected backward-compatible aliases for the old setting-map surface.

## Key Files

- `packages/aqc-file-engine-core/src/types.rs`
- `packages/file-types/toml/aqc-rust-toolchain-toml-engine/src/requirement/model.rs`
- `packages/file-types/toml/aqc-rust-toolchain-toml-engine/src/requirement/merge.rs`
- `packages/file-types/toml/aqc-rust-toolchain-toml-engine/src/reconcile/settings.rs`
- `packages/file-types/toml/aqc-rust-toolchain-toml-engine/src/reconcile/settings_support.rs`
- `packages/file-types/toml/aqc-rust-toolchain-toml-engine/tests/behavior.rs`

## Verification

- `cargo test --manifest-path packages/file-types/toml/aqc-rust-toolchain-toml-engine/Cargo.toml --all-targets`
- `specular verify specs/2026-07-06-144327-toolchain-redesign.spec.json` from the Shackles repo

## Next Steps

- Publish the AQC crate set before running plain Shackles Cargo commands without local `[patch.crates-io]`.
