Summary:
- Split `aqc-cargo-toml-engine`'s `requirement/cargo_toml.rs` aggregate into a nested module tree.
- Moved public model types, merge orchestration, resolve helpers, and dependency conflict checks into separate files.
- Updated public-field waivers to the new model path and added a waiver for the dependency glob conflict aggregate.

Decisions made:
- Fixed file-size and import-count errors by separating roles instead of waiving structural findings.
- Kept the existing public export path through `requirement::cargo_toml` so callers do not need to change imports.
- Replaced broad type imports in the model with module aliases to reduce import count while keeping field ownership visible.

Verification:
- `cargo fmt -p aqc-cargo-toml-engine`
- `cargo test -p aqc-cargo-toml-engine`
- `g3rs validate workspace --path . 2>&1 | rg 'aqc-cargo-toml-engine/src/requirement/cargo_toml|aqc-cargo-toml-engine/src/requirement/cargo_toml.rs|^\[Error\]'`

Remaining issues:
- `aqc-cargo-toml-engine/src/reconcile/dependencies.rs` still has file-size and import-count g3rs errors.
- `aqc-cargo-toml-engine/src/reconcile/workspace_fields.rs` still has import-count g3rs errors.
- `aqc-cargo-toml-engine/tests/merge.rs` still has file-size g3rs errors.
- `aqc-rustfmt-toml-engine` still has separate g3rs errors and existing uncommitted changes.

Key files for context:
- `packages/file-types/toml/aqc-cargo-toml-engine/src/requirement/cargo_toml/mod.rs`
- `packages/file-types/toml/aqc-cargo-toml-engine/src/requirement/cargo_toml/model.rs`
- `packages/file-types/toml/aqc-cargo-toml-engine/src/requirement/cargo_toml/merge.rs`
- `packages/file-types/toml/aqc-cargo-toml-engine/src/requirement/cargo_toml/resolve.rs`
- `packages/file-types/toml/aqc-cargo-toml-engine/src/requirement/cargo_toml/conflicts.rs`
- `guardrail3-rs.toml`

Next steps:
- Split `reconcile/dependencies.rs` by role, then clean up `reconcile/workspace_fields.rs` imports.
- Split the large cargo merge test file only after preserving test names and coverage.
