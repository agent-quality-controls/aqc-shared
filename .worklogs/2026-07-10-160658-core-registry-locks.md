# Core Registry Locks

## Summary

Refreshed the TOML and text engine-core lockfiles after `aqc-file-engine-core 0.4.1` became available from crates.io.

## Decisions Made

- Published dependencies are resolved from crates.io for release packaging.
- Local patches remain confined to the isolated repository gate.

## Key Files

- `packages/file-types/toml/aqc-toml-engine-core/Cargo.lock`
- `packages/file-types/text/aqc-text-engine-core/Cargo.lock`

## Next Steps

- Publish both engine-core crates.
- Refresh and publish the concrete file-engine tier.
