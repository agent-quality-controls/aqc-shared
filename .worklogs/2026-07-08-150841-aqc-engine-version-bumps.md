# AQC Engine Version Bumps

## Summary

Bumped changed concrete AQC TOML file-engine crate versions so the repo-hooks engine-core API changes can be published without reusing already-published crate versions.

## Decisions Made

- `aqc-cargo-toml-engine` moves to `0.3.5`.
- `aqc-clippy-toml-engine` moves to `0.3.5`.
- `aqc-deny-toml-engine` moves to `0.1.2`.
- `aqc-rust-toolchain-toml-engine` moves to `0.3.4`.
- `aqc-rustfmt-toml-engine` moves to `0.3.4`.

## Key Files

- `packages/file-types/toml/*/Cargo.toml`

## Next Steps

- Publish AQC crates in dependency order.
- Update Shackles manifests to depend on the publishable AQC versions.
