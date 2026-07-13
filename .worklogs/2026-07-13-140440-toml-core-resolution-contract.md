# Summary

Released `aqc-toml-engine-core` 0.6.0 against the successful-or-conflicted resolution contract in `aqc-file-engine-core` 0.6.0.

# Decisions made

- Kept TOML mechanics unchanged because the breaking contract belongs to file-engine core and concrete engine merge APIs.
- Regenerated the lockfile from crates.io so no local path resolution can mask the published dependency contract.

# Key files for context

- `packages/file-types/toml/aqc-toml-engine-core/Cargo.toml`
- `packages/file-types/toml/aqc-toml-engine-core/Cargo.lock`
- `specs/resolution-contract-cleanup.spec.json`

# Next steps

- Publish `aqc-toml-engine-core` 0.6.0.
- Migrate and publish the concrete file engines against this generation.
