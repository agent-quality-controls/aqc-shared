# Clean Cargo Engine Lockfile

## Summary

Removed `[[patch.unused]]` entries from the `aqc-cargo-toml-engine` lockfile after local Shackles verification.
The lockfile should not record temporary downstream verification patches.

## Decisions Made

- Removed only unused patch metadata.
- Kept the `aqc-cargo-toml-engine v0.3.4` version and dependency lock state.

## Key Files For Context

- `packages/file-types/toml/aqc-cargo-toml-engine/Cargo.lock`

## Verification

- `rg '^\\[\\[patch\\.unused\\]\\]' packages/file-types/toml/aqc-cargo-toml-engine/Cargo.lock` returns no matches.

## Next Steps

- Avoid committing temporary Cargo home patch metadata in future verification runs.
