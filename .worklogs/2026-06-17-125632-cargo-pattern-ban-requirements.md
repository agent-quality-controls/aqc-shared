Summary
- Added generic pattern-ban requirements to file engine core.
- Added Cargo dependency package-pattern bans for dependency tables, workspace dependencies, target dependency scopes, and patch tables.
- Cargo reconcile now removes matching packages by effective package identity while preserving conflicted required dependencies.

Decisions made
- Put the generic pattern-ban container and resolver in `aqc-file-engine-core`.
- Keep Cargo package-pattern semantics in `aqc-cargo-toml-engine`.
- Use `globset` instead of custom glob matching.
- Store required-vs-pattern conflicts as conflict blocks instead of deleting either side from the resolved requirement set.
- Skip a conflicted pattern as a whole during reconcile so a contradictory policy cannot remove a required package.

Key files for context
- `packages/aqc-file-engine-core/src/merge.rs`
- `packages/aqc-file-engine-core/src/lib.rs`
- `packages/file-types/toml/aqc-cargo-toml-engine/src/requirement/cargo_toml.rs`
- `packages/file-types/toml/aqc-cargo-toml-engine/src/requirement/dependencies.rs`
- `packages/file-types/toml/aqc-cargo-toml-engine/src/reconcile/dependencies.rs`
- `packages/file-types/toml/aqc-cargo-toml-engine/src/reconcile/patch.rs`
- `packages/file-types/toml/aqc-cargo-toml-engine/tests/merge.rs`

Verification
- `cargo test -p aqc-cargo-toml-engine`
- Focused conflict-preservation regression tests in `aqc-cargo-toml-engine/tests/merge.rs`
- `git diff --check`

Known issue
- Broad `cargo clippy -p aqc-cargo-toml-engine --all-targets -- -D warnings` is still blocked by wider existing lint gates in this repo.

Next steps
- Add direct workspace and patch conflict tests if more assurance is needed beyond the shared `apply_set` path.
- Consume the new adapter fields from Specular when that repository is in scope.
