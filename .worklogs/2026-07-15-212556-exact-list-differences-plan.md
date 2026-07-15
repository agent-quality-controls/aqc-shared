# Exact List Differences Plan

## Summary
Designed the AQC prerequisite for member-specific exact-list findings across JSON, TOML, and YAML. The plan keeps the established list algebra and adds one universal read-only difference result.

## Decisions Made
- Keep only `contains`, `excludes`, and `exact`; do not add allowed or closed assertions.
- Compute multiset membership differences and order-only mismatches once in file-engine core.
- Keep finding keys, rendering, mutation, and shape errors in format engines.
- Add presence-aware TOML entry points that delegate to the existing known-present reconciler.
- Preserve compatible exact-plus-contains/excludes findings separately so each policy retains attribution under one waiver identity.
- Use patch releases accepted by existing downstream ranges; do not force unrelated package migrations.

## Key Files For Context
- `.plans/2026-07-15-211236-exact-list-differences.md`
- `packages/aqc-file-engine-core/src/merge/model.rs`
- `packages/aqc-file-engine-core/src/merge/lists.rs`
- `packages/file-types/json/aqc-json-file-engine/src/runtime/reconcile.rs`
- `packages/file-types/toml/aqc-toml-engine-core/src/lists.rs`
- `packages/file-types/yaml/aqc-pnpm-workspace-yaml-engine/src/runtime/reconcile/apply.rs`

## Next Steps
- Extract an AQC Specular spec from the frozen plan.
- Implement core, JSON, TOML, and YAML changes in dependency order.
- Prove all three format families before releasing the patch chain.
