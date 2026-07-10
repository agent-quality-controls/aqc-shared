# Pathless Engine Release

## Summary

Published the complete pathless AQC engine chain and refreshed every dependent workspace lockfile against crates.io. Removed completed plans, temporary Specular contracts, and verifiers, including the obsolete contract that required the deleted Git hooks engine.

## Decisions Made

- Published `aqc-file-engine-core 0.4.0`.
- Published `aqc-toml-engine-core 0.4.0` and `aqc-text-engine-core 0.2.0`.
- Published Cargo, Clippy, rust-toolchain, and rustfmt TOML engines at `0.4.0`.
- Published `aqc-deny-toml-engine 0.2.0`.
- Kept worklogs as the historical record and removed completed implementation plans/specs.
- Left the unrelated remote `release-plz-2026-06-26T22-49-53Z` branch unchanged for explicit user adjudication.

## Key Files For Context

- `packages/aqc-file-engine-core/src/engine.rs`
- `packages/file-types/text/aqc-text-engine-core/src/engine.rs`
- `packages/file-types/toml/aqc-toml-engine-core/src/lib.rs`
- `.worklogs/2026-07-10-004438-pathless-file-engines.md`
- `.worklogs/2026-07-10-105324-concrete-engine-release-lockfiles.md`

## Next Steps

- Keep future file engines pathless and preserve runner ownership of file placement.
- Decide separately whether to delete the orphaned remote release-plz branch.
