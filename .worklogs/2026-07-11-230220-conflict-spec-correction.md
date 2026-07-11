# Conflict Spec Correction

## Summary

Updated the exact-item Specular release gate for the conflict short-circuit patch release.

## Decisions Made

- The spec requires `aqc-file-engine-core 0.5.1`, which contains the conflict-safe erased reconciler.

## Key Files For Context

- `specs/create-only-init-and-exact-items.spec.json`
- `packages/aqc-file-engine-core/src/engine.rs`

## Next Steps

- Rerun AQC Specular conformance after downstream lockfiles refresh.
