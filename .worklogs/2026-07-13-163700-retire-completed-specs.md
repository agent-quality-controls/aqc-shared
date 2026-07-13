# Retire Completed Specs

## Summary

Removed completed implementation-time Specular specs, coverage maps, and custom verifiers after conformance and adversarial convergence.

## Decisions Made

- Kept package tests, Cargo-deny configuration, and repository hooks as persistent gates.
- Removed only throwaway specs whose plans are complete.

## Key Files For Context

- `packages/aqc-file-engine-core`
- `packages/file-types/toml/aqc-cargo-toml-engine`
- `packages/file-types/toml/aqc-clippy-toml-engine`

## Next Steps

- Push the completed Task 1 history.
