# Summary

Exported `asserted_items` through the `aqc-file-engine-core` crate facade and prepared version 0.6.3. This makes the universal resolved-item inspection operation usable by downstream engines through the supported public API.

# Decisions made

- Kept `ResolvedItemRequirements` as plain data and exposed inspection as a free core operation.
- Corrected the crate facade instead of importing through the internal `merge` module.
- Bumped the patch version because 0.6.2 was already published without the required root export.

# Key files for context

- `packages/aqc-file-engine-core/src/lib.rs`
- `packages/aqc-file-engine-core/src/merge/items.rs`
- `packages/aqc-file-engine-core/Cargo.toml`

# Next steps

- Publish 0.6.3.
- Update and verify downstream AQC engine lockfiles.
- Release Cargo and Clippy engine patch versions containing exact-item glob conflict checks.
