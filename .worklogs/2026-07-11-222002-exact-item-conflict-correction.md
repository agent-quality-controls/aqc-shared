# Exact Item Conflict Correction

## Summary

Separated exact members from explicit required members during core resolution and added exhaustive exact-item merge tests.

## Decisions Made

- Exact members populate only resolved exact state.
- Explicit required state remains explicit, preventing duplicate required/forbidden conflicts.
- Exact and required values still compose together for identities named by exact.

## Key Files For Context

- `packages/aqc-file-engine-core/src/merge/items.rs`
- `packages/aqc-file-engine-core/tests/exact_items.rs`

## Next Steps

- Publish the corrected `0.5.0` core release.
- Verify all dependent engines against it.
