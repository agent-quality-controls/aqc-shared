# Exact Conflict Attribution

## Summary

Preserved requirement messages in same-identity item value conflicts and prepared `aqc-file-engine-core 0.5.2`.

## Decisions Made

- `compose_item_by` reports each assertion's message instead of replacing it with the generic word `required`.
- Exact and required value conflicts now retain their distinct repair instructions and provenance.

## Key Files For Context

- `packages/aqc-file-engine-core/src/merge/items.rs`
- `packages/aqc-file-engine-core/tests/exact_items.rs`

## Next Steps

- Publish core `0.5.2`.
- Release corrected TOML exact reconciliation against it.
