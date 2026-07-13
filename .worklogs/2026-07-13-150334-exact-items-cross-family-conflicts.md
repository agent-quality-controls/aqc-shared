# Exact Items Cross-Family Conflicts

## Summary

Made cross-family conflict checks inspect items asserted through either `required` or `exact` collections.

## Decisions Made

- Add one universal `ResolvedItemRequirements::asserted_items` view in file-engine core.
- Prefer exact item resolutions when the same identity also appears in required assertions.
- Reuse the view in Cargo dependency-package glob and Clippy path-glob conflicts.
- Strengthen scope and resolved-root verifiers found weak by adversarial review.

## Key Files For Context

- `packages/aqc-file-engine-core/src/merge/model.rs`
- `packages/file-types/toml/aqc-cargo-toml-engine/src/requirement/cargo_toml/conflicts.rs`
- `packages/file-types/toml/aqc-clippy-toml-engine/src/requirement/disallowed.rs`
- `specs/verifiers/verify_resolution_contract_cleanup.py`

## Next Steps

1. Publish `aqc-file-engine-core` 0.6.2.
2. Refresh Cargo and Clippy engine lockfiles and verify exact-only regressions.
