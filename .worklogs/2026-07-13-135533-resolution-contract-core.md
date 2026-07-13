# Resolution Contract Core

## Summary

Changed the universal erased reconciliation contract to accept successful-or-conflicted `Result` merges. Added contract tests proving conflict byte preservation, attribution, ordering, and that reconciliation is never called after a failed merge.

## Decisions Made

- Use `Result<ResolvedRequirements, Vec<ConflictEntry>>` without a compatibility tuple API.
- Preserve the existing `Provenance.policy` projection into finding contributors.
- Keep empty erased input behavior unchanged.
- Release this breaking pre-1.0 contract as `aqc-file-engine-core 0.6.0` before dependent format cores and engines.

## Key Files For Context

- `packages/aqc-file-engine-core/src/engine.rs`
- `packages/aqc-file-engine-core/tests/public_contract.rs`
- `specs/resolution-contract-cleanup.spec.json`
- `/Users/tartakovsky/Projects/agent-quality-controls/shackles/.plans/2026-07-13-124615-resolution-contract-cleanup.md`

## Next Steps

- Publish `aqc-file-engine-core 0.6.0`.
- Resolve and release the format cores and concrete engines against it.
