Summary:
- Split the large `aqc-cargo-toml-engine/tests/merge.rs` integration test into focused files by behavior area.
- Added a shared `tests/common.rs` helper module and kept the original merge test names across the split files.
- Cleared the cargo-engine error-level g3rs findings; remaining errors are now in `aqc-rustfmt-toml-engine`.

Decisions made:
- Used normal `mod common;` from split test files because g3rs forbids `#[path]` and `include!`.
- Kept `tests/common.rs` as a standalone integration test with a helper self-check so it compiles cleanly under `-D warnings`.
- Added reasoned module-level allows on each split test file because each file uses a subset of the shared helper module.

Verification:
- `cargo fmt -p aqc-cargo-toml-engine`
- `cargo test -p aqc-cargo-toml-engine`
- `g3rs validate workspace --path . 2>&1 | rg 'aqc-cargo-toml-engine/tests|aqc-cargo-toml-engine/src|^\[Error\]'`

Remaining issues:
- `aqc-rustfmt-toml-engine/src/reconcile/settings.rs` still has a file-size g3rs error.
- `aqc-rustfmt-toml-engine/src/requirement.rs` still has a public-field g3rs error.
- `aqc-rustfmt-toml-engine/tests/reconcile.rs` still has a file-size g3rs error.

Key files for context:
- `packages/file-types/toml/aqc-cargo-toml-engine/tests/common.rs`
- `packages/file-types/toml/aqc-cargo-toml-engine/tests/dependency_identity.rs`
- `packages/file-types/toml/aqc-cargo-toml-engine/tests/dependency_globs.rs`
- `packages/file-types/toml/aqc-cargo-toml-engine/tests/dependency_fields.rs`
- `packages/file-types/toml/aqc-cargo-toml-engine/tests/lints_features_tables.rs`
- `packages/file-types/toml/aqc-cargo-toml-engine/tests/attribution_scalars.rs`

Next steps:
- Review the existing rustfmt-engine worktree changes, then split or waive the remaining rustfmt g3rs findings.
