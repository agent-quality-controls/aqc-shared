# Engine Registry Locks

## Summary

Refreshed all concrete TOML engine lockfiles after the shared engine cores became available from crates.io.

## Decisions Made

- Concrete engine release archives resolve both core tiers from crates.io.
- The five concrete engines remain independent workspaces and releases.

## Key Files

- `packages/file-types/toml/aqc-cargo-toml-engine/Cargo.lock`
- `packages/file-types/toml/aqc-clippy-toml-engine/Cargo.lock`
- `packages/file-types/toml/aqc-deny-toml-engine/Cargo.lock`
- `packages/file-types/toml/aqc-rust-toolchain-toml-engine/Cargo.lock`
- `packages/file-types/toml/aqc-rustfmt-toml-engine/Cargo.lock`

## Next Steps

- Publish all concrete engines.
- Refresh downstream Shackles lockfiles against their published AQC dependencies.
