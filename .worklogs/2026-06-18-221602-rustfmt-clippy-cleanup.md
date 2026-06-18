Summary:
- Made `aqc-rustfmt-toml-engine` pass its package clippy gate.
- Added rustfmt requirement type aliases, documented private helpers, exported public aliases, and fixed test indexing/shadowing/type-complexity findings.

Decisions made:
- Used type aliases for repeated resolved rustfmt requirement shapes instead of suppressing `type_complexity`.
- Added doc comments for private helper functions because the workspace denies `missing_docs_in_private_items`.
- Kept the exhaustive rustfmt setting key map in one function with a localized `#[expect(clippy::too_many_lines)]` and reason.

Verification:
- `cargo fmt -p aqc-rustfmt-toml-engine`
- `cargo clippy -p aqc-rustfmt-toml-engine --all-targets --all-features -- -D warnings`
- `cargo test -p aqc-rustfmt-toml-engine`

Remaining issues:
- The full workspace clippy gate still has failures in other crates, mainly cargo and clippy TOML engines.

Key files for context:
- `packages/file-types/toml/aqc-rustfmt-toml-engine/src/requirement.rs`
- `packages/file-types/toml/aqc-rustfmt-toml-engine/src/reconcile/settings/*.rs`
- `packages/file-types/toml/aqc-rustfmt-toml-engine/tests/merge.rs`
- `packages/file-types/toml/aqc-rustfmt-toml-engine/tests/reconcile_ignore_closed.rs`

Next steps:
- Run clippy for the clippy TOML engine, then cargo TOML engine, and fix those batches separately.
