# Concrete Engine Release Lockfiles

## Summary

Refreshed the five changed concrete TOML engine lockfiles against the published AQC `0.4.0` core release line. Each workspace now packages from registry dependencies with `--locked`.

## Decisions Made

- Kept all engine manifest versions and source unchanged.
- Regenerated Cargo, Clippy, deny, rust-toolchain, and rustfmt engine lockfiles only after both core layers were published.
- Preserved independent workspace lockfiles and release order.

## Key Files For Context

- `packages/file-types/toml/aqc-cargo-toml-engine/Cargo.lock`
- `packages/file-types/toml/aqc-clippy-toml-engine/Cargo.lock`
- `packages/file-types/toml/aqc-deny-toml-engine/Cargo.lock`
- `packages/file-types/toml/aqc-rust-toolchain-toml-engine/Cargo.lock`
- `packages/file-types/toml/aqc-rustfmt-toml-engine/Cargo.lock`

## Next Steps

- Package and publish all five concrete engines.
- Publish the downstream Shackles `0.2.0` crate chain.
