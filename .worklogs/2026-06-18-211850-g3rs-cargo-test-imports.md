## Summary

Reduced Cargo engine test import counts by switching tests to crate aliases instead of importing every requirement and core type into the file namespace.

## Decisions Made

- Replaced large test import lists with `cargo::...` and `engine_core::...` qualified names.
- Kept `use engine_core::Engine` so trait-method calls to `reconcile` stay readable.
- Kept `use globset as _` and `use toml_edit as _` because test crates compile with `unused-crate-dependencies` denied.

## Verification

- `cargo fmt -p aqc-cargo-toml-engine`
- `cargo test -p aqc-cargo-toml-engine`
- `g3rs validate workspace --path .` still fails on remaining large-file/source import findings and Clippy TOML findings. The Cargo test import-count findings are gone.

## Key Files For Context

- `packages/file-types/toml/aqc-cargo-toml-engine/tests/contract.rs`
- `packages/file-types/toml/aqc-cargo-toml-engine/tests/contract_tables.rs`
- `packages/file-types/toml/aqc-cargo-toml-engine/tests/merge.rs`

## Next Steps

- Split the oversized Cargo `tests/merge.rs`.
- Split `src/reconcile/dependencies.rs`.
- Split `src/requirement/cargo_toml.rs`.
- Reduce imports in `src/reconcile/workspace_fields.rs`.
