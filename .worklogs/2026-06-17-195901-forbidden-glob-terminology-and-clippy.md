Summary
- Renamed the generic glob forbid API to `ForbiddenGlob*` and `globs`.
- Added Clippy TOML forbidden path globs for `disallowed-methods`, `disallowed-types`, and `disallowed-macros`.
- Cargo dependency forbidden package globs keep the same behavior under the cleaned terminology.

Decisions made
- Use AQC terms `forbidden` and `glob` for library-owned matching rules.
- Keep file-native terms such as Clippy `disallowed-*` when modeling Clippy's file format.
- Keep exact item terminology separate from glob terminology; exact item APIs still use their existing `banned` field names.
- Keep glob compilation and file scanning inside each file engine, with merge and attribution in engine core.

Key files for context
- `packages/aqc-file-engine-core/src/merge.rs`
- `packages/file-types/toml/aqc-cargo-toml-engine/src/requirement/dependencies.rs`
- `packages/file-types/toml/aqc-cargo-toml-engine/src/reconcile/dependencies.rs`
- `packages/file-types/toml/aqc-cargo-toml-engine/src/reconcile/patch.rs`
- `packages/file-types/toml/aqc-clippy-toml-engine/src/requirement.rs`
- `packages/file-types/toml/aqc-clippy-toml-engine/src/reconcile/bans.rs`
- `packages/file-types/toml/aqc-clippy-toml-engine/tests/merge.rs`

Verification
- `cargo fmt -p aqc-file-engine-core -p aqc-cargo-toml-engine -p aqc-clippy-toml-engine`
- `cargo test -p aqc-cargo-toml-engine`
- `cargo test -p aqc-clippy-toml-engine`
- `git diff --check`

Next steps
- If exact item terminology is migrated from `banned` to `forbidden`, do it as a separate exact-item API pass.
