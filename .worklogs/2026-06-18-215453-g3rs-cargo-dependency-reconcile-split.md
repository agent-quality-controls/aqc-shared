Summary:
- Split `aqc-cargo-toml-engine` dependency reconciliation into a facade module plus apply, required-entry, removal, and TOML dependency I/O helpers.
- Removed the g3rs file-size and import-count errors from the old `reconcile/dependencies.rs`.

Decisions made:
- Kept `crate::reconcile::dependencies::{apply, apply_set, SetRule}` as the internal API used by patch/workspace dependency reconciliation.
- Moved TOML parsing/rendering into `spec_io.rs` so reconciliation logic no longer owns dependency serialization details.
- Moved removal planning into `removals.rs` and required-entry write logic into `required.rs`.

Verification:
- `cargo fmt -p aqc-cargo-toml-engine`
- `cargo test -p aqc-cargo-toml-engine`
- `g3rs validate workspace --path . 2>&1 | rg 'aqc-cargo-toml-engine/src/reconcile/dependencies|aqc-cargo-toml-engine/src/reconcile/workspace_fields|aqc-cargo-toml-engine/tests/merge|^\[Error\]'`

Remaining issues:
- `aqc-cargo-toml-engine/src/reconcile/workspace_fields.rs` still has an import-count g3rs error.
- `aqc-cargo-toml-engine/tests/merge.rs` still has a file-size g3rs error.
- `aqc-rustfmt-toml-engine` still has separate g3rs errors and existing uncommitted changes.

Key files for context:
- `packages/file-types/toml/aqc-cargo-toml-engine/src/reconcile/dependencies/mod.rs`
- `packages/file-types/toml/aqc-cargo-toml-engine/src/reconcile/dependencies/apply.rs`
- `packages/file-types/toml/aqc-cargo-toml-engine/src/reconcile/dependencies/required.rs`
- `packages/file-types/toml/aqc-cargo-toml-engine/src/reconcile/dependencies/removals.rs`
- `packages/file-types/toml/aqc-cargo-toml-engine/src/reconcile/dependencies/spec_io.rs`

Next steps:
- Reduce `workspace_fields.rs` import count without changing reconciliation behavior.
- Split the large cargo merge test file by behavior group.
