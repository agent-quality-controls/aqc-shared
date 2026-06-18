Summary:
- Reduced `aqc-cargo-toml-engine/src/reconcile/workspace_fields.rs` import count by switching individual imports to module aliases.
- Kept the reconciliation logic and behavior unchanged.

Decisions made:
- Used `core_types`, `toml`, `util`, and `req` aliases instead of splitting the file because g3rs only reported import count, not file size.
- Left warning-level findings in other split files unchanged.

Verification:
- `cargo fmt -p aqc-cargo-toml-engine`
- `cargo test -p aqc-cargo-toml-engine`
- `g3rs validate workspace --path . 2>&1 | rg 'workspace_fields.rs|aqc-cargo-toml-engine/tests/merge.rs|^\[Error\]'`

Remaining issues:
- `aqc-cargo-toml-engine/tests/merge.rs` still has a file-size g3rs error.
- `aqc-rustfmt-toml-engine` still has separate g3rs errors and existing uncommitted changes.

Key files for context:
- `packages/file-types/toml/aqc-cargo-toml-engine/src/reconcile/workspace_fields.rs`

Next steps:
- Split the large cargo merge test file by behavior group while preserving test names.
