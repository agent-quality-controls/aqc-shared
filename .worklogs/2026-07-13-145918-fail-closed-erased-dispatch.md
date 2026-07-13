# Fail-Closed Erased Dispatch

## Summary

Made erased engine dispatch fail closed when any routed requirement has the wrong concrete type.

## Decisions Made

- Preserve the empty requirement behavior only for an empty input slice.
- Return one hard `InternalError` without merge or reconciliation for wrong or mixed concrete types.
- Preserve current bytes while reporting the internal routing failure.

## Key Files For Context

- `packages/aqc-file-engine-core/src/engine.rs`
- `packages/aqc-file-engine-core/tests/public_contract.rs`
- `specs/resolution-contract-cleanup.spec.json`

## Next Steps

1. Publish `aqc-file-engine-core` 0.6.1.
2. Refresh dependent registry lockfiles and rerun both repository specs.
