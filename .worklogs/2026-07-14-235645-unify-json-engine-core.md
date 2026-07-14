# Unify JSON Engine Core

## Summary

Consolidated strict JSON and TypeScript JSONC mechanics into one lossless
`aqc-json-engine-core`. Removed `aqc-jsonc-engine-core` and migrated package
JSON, tsconfig, release metadata, dependency gates, specs, and fixtures.

## Decisions

- One configurable JSON-family core owns parsing, CST edits, scalar access,
  rendering, and diagnostics.
- Concrete engines select explicit `JsonParseOptions`.
- Strict package JSON and TypeScript JSONC remain separate engine contracts.
- Invalid parent replacement is explicit through `NonObjectParentAction`.
- No alias, compatibility crate, or duplicate API remains.

## Key Files

- `.plans/2026-07-14-174701-unify-json-engine-core.md`
- `packages/file-types/json/aqc-json-engine-core/src/runtime/parse.rs`
- `packages/file-types/json/aqc-json-engine-core/src/runtime/extensions.rs`
- `packages/file-types/json/aqc-json-engine-core/src/types/object.rs`
- `packages/file-types/json/aqc-json-engine-core/tests/core_contract.rs`
- `specs/json-engine-core-consolidation.spec.json`

## Verification

- Consolidation Specular spec passes.
- PNPM and TSC AQC specs pass.
- PNPM and TSC AQC Fixture3 suites match.
- Format, Clippy, package, deny, release, and boundary gates pass.
- Final independent adversarial review reported no findings.

## Next Steps

- Release the AQC JSON crates when a coordinated crates.io release is requested.
