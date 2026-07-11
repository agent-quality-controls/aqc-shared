# Conflict Short Circuit

## Summary

Changed universal erased reconciliation to stop before file reconciliation when requirement merge reports conflicts.

## Decisions Made

- Conflicted requirements return unchanged current bytes and only conflict findings.
- Partially resolved requirements never produce contradictory mismatch findings or candidate edits.
- The behavior is universal in `aqc-file-engine-core`, not repeated in concrete engines or runners.

## Key Files For Context

- `packages/aqc-file-engine-core/src/engine.rs`
- `packages/aqc-file-engine-core/tests/public_contract.rs`

## Next Steps

- Publish `aqc-file-engine-core 0.5.1`.
- Refresh downstream lockfiles and rerun the conflict fixture.
