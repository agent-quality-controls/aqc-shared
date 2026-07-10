# Core Release Lockfiles

## Summary

Refreshed the TOML and text engine-core lockfiles against the published `aqc-file-engine-core 0.4.0`. This removes local-patch lock state that prevented registry packaging with `--locked`.

## Decisions Made

- Kept all manifest versions unchanged.
- Regenerated only the two direct dependent lockfiles after the base core became available on crates.io.
- Kept source and public APIs unchanged during release work.

## Key Files For Context

- `packages/file-types/toml/aqc-toml-engine-core/Cargo.lock`
- `packages/file-types/text/aqc-text-engine-core/Cargo.lock`
- `packages/aqc-file-engine-core/Cargo.toml`

## Next Steps

- Publish `aqc-toml-engine-core 0.4.0` and `aqc-text-engine-core 0.2.0`.
- Refresh, package, and publish the concrete TOML engines.
