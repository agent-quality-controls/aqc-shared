Summary:
- Split `aqc-clippy-toml-engine` requirement code into a `requirement/` module tree to remove the old g3rs file-size and import-count findings.
- Kept the public requirement API intact by re-exporting the resolved aggregate and forbidden-glob conflict block from the crate root.
- Added targeted waivers for public-field data records that are API shapes, not behavior objects.

Decisions made:
- Moved requirement model types, merge logic, ban/glob types, and scalar assertion logic into separate files instead of waiving file-size/import-count errors.
- Waived `ClippyPathGlob` and `ClippyForbiddenGlobConflictBlocks` because their public fields are plain input/output data contracts.
- Left unrelated rustfmt-engine worktree changes unstaged.

Verification:
- `cargo fmt -p aqc-clippy-toml-engine`
- `cargo test -p aqc-clippy-toml-engine`
- `g3rs validate workspace --path . 2>&1 | rg 'aqc-clippy-toml-engine/src/requirement|aqc-clippy-toml-engine/src/lib.rs|^\[Error\]'`

Remaining issues:
- `aqc-cargo-toml-engine/src/reconcile/dependencies.rs` still has file-size and import-count g3rs errors.
- `aqc-cargo-toml-engine/src/reconcile/workspace_fields.rs` still has import-count g3rs errors.
- `aqc-cargo-toml-engine/src/requirement/cargo_toml.rs` still has file-size and import-count g3rs errors.
- `aqc-cargo-toml-engine/tests/merge.rs` still has file-size g3rs errors.
- `aqc-rustfmt-toml-engine` still has g3rs errors, plus existing uncommitted changes.

Key files for context:
- `packages/file-types/toml/aqc-clippy-toml-engine/src/requirement/mod.rs`
- `packages/file-types/toml/aqc-clippy-toml-engine/src/requirement/model.rs`
- `packages/file-types/toml/aqc-clippy-toml-engine/src/requirement/merge.rs`
- `packages/file-types/toml/aqc-clippy-toml-engine/src/requirement/bans.rs`
- `packages/file-types/toml/aqc-clippy-toml-engine/src/requirement/scalar.rs`
- `guardrail3-rs.toml`

Next steps:
- Split the remaining cargo TOML engine files that g3rs reports as too large or import-heavy.
- Review rustfmt-engine changes before deciding which are existing work and which are part of the g3rs cleanup.
