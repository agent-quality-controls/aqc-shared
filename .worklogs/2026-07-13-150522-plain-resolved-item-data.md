# Plain Resolved Item Data

## Summary

Kept resolved item structs as plain data while retaining the universal asserted-item operation in file-engine core.

## Decisions Made

- Export `asserted_items` as a core merge function.
- Do not attach behavior to `ResolvedItemRequirements`.
- Keep Cargo and Clippy call sites on the same core operation.

## Key Files For Context

- `packages/aqc-file-engine-core/src/merge/items.rs`
- `packages/aqc-file-engine-core/src/merge/model.rs`
- `packages/aqc-file-engine-core/src/merge/mod.rs`

## Next Steps

- Publish file-engine core 0.6.2 and consume it in affected engines.
