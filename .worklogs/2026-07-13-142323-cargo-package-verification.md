# Summary

Corrected three Cargo merge helper signatures that retained an unqualified `ConflictEntry` after import consolidation.

# Decisions made

- Kept the import count within the repository gate by using the fully qualified core type in all four affected signatures.
- Re-ran the complete Cargo engine test, Clippy, and package verification gates before release.

# Key files for context

- `packages/file-types/toml/aqc-cargo-toml-engine/src/requirement/cargo_toml/merge.rs`

# Next steps

- Publish the Cargo engine and remaining engine generation.
