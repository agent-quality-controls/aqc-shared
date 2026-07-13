# Summary

Prepared Cargo and Clippy engine 0.6.1 releases with exact-only item conflict coverage, and advanced all affected AQC lockfiles to `aqc-file-engine-core` 0.6.3.

# Decisions made

- Released Cargo and Clippy because their reconciliation behavior changed.
- Kept other AQC crate versions unchanged because only their resolved lock dependency changed.
- Used the universal core `asserted_items` operation rather than engine-local inspection helpers.

# Key files for context

- `packages/file-types/toml/aqc-cargo-toml-engine/src/requirement/cargo_toml/conflicts.rs`
- `packages/file-types/toml/aqc-cargo-toml-engine/tests/dependency_globs.rs`
- `packages/file-types/toml/aqc-clippy-toml-engine/src/requirement/disallowed.rs`
- `packages/file-types/toml/aqc-clippy-toml-engine/tests/merge.rs`

# Next steps

- Publish Cargo and Clippy engine 0.6.1.
- Verify all affected AQC workspaces and the resolution-contract spec.
- Refresh Shackles locks against the patch releases.
