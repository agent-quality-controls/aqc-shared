# Shakts Dependency Boundaries

## Summary

Extended existing AQC workspace deny files with the new downstream `shakts` CLI and `shakts-hooks-policy` identities.

## Decisions Made

- Changed only Cargo-deny configuration; no AQC source, API, dependency, or behavior changed.
- Preserved the rule that AQC crates cannot depend on Shackles products or policies.

## Key Files For Context

- `packages/aqc-file-engine-core/deny.toml`
- `packages/file-types/text/aqc-text-engine-core/deny.toml`
- `../shackles/scripts/check-dependency-boundaries.py`

## Next Steps

- Keep deny identities synchronized when new Shackles packages are added.
