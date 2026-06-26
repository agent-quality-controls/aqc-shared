# Shakrs Engine Surface Support

## Summary

Updated AQC TOML engine public surfaces needed by the Shakrs migration.
Cargo lint-table requirement construction now lives in the Cargo TOML engine instead of a Shackles adapter.

## Decisions Made

- Reexported supported `aqc-file-engine-core` assertion types from Cargo, Clippy, and Rustfmt TOML engines.
- Added `cargo_lint_table_requirements` to `aqc-cargo-toml-engine`.
- Kept the helper engine-owned: it returns `CargoTomlRequirements` and has no Shackles, policy, adapter, provenance, or boxed-output concepts.
- Did not introduce engine-to-engine dependencies or any dependency from AQC shared back into Shackles.

## Key Files For Context

- `packages/file-types/toml/aqc-cargo-toml-engine/src/lib.rs`
- `packages/file-types/toml/aqc-cargo-toml-engine/src/requirement/lint_tables.rs`
- `packages/file-types/toml/aqc-cargo-toml-engine/src/requirement/mod.rs`
- `packages/file-types/toml/aqc-clippy-toml-engine/src/lib.rs`
- `packages/file-types/toml/aqc-rustfmt-toml-engine/src/lib.rs`

## Verification

- `cargo check --manifest-path packages/file-types/toml/aqc-cargo-toml-engine/Cargo.toml --all-targets`
- `cargo check --manifest-path packages/file-types/toml/aqc-clippy-toml-engine/Cargo.toml --all-targets`
- `cargo check --manifest-path packages/file-types/toml/aqc-rustfmt-toml-engine/Cargo.toml --all-targets`

## Next Steps

- Publish the affected AQC crates before publishing Shakrs crates that depend on these public surfaces.
