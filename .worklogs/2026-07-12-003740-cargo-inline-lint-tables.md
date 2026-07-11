# Cargo inline lint tables

## Summary

The Cargo engine now recognizes standard and inline TOML tables as package-local lint-table identities. Exact-empty and required-table requirements therefore match every Cargo-supported table syntax.

## Decisions made

- Kept syntax recognition in the Cargo engine because `[lints.<tool>]` identity is Cargo-specific.
- Reused one predicate for required, forbidden, and exact reconciliation.
- Added direct regressions for required and exact-empty behavior.

## Key files for context

- `packages/file-types/toml/aqc-cargo-toml-engine/src/reconcile/package_lint_tables.rs`
- `packages/file-types/toml/aqc-cargo-toml-engine/tests/package_lint_tables.rs`
- `packages/file-types/toml/aqc-rust-toolchain-toml-engine/tests/behavior.rs`

## Next steps

- Publish `aqc-cargo-toml-engine 0.5.2` and refresh downstream locks.
