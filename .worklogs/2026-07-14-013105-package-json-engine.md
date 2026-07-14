# Package JSON Engine

## Summary

Added the pathless `aqc-package-json-engine` for resolving and reconciling package-manager declarations in Package JSON bytes. The engine reuses file-engine-core assertions and JSON-engine-core parsing, findings, and rendering.

## Decisions Made

- Modeled Package JSON field structure in the engine without pnpm policy meaning.
- Preserved unrelated object members and changed only addressed fields.
- Kept check-only assertions non-initializable and deterministic requirements writable.
- Exposed one unresolved root and one resolved root; only the engine composes conflicts.

## Key Files For Context

- `packages/file-types/json/aqc-package-json-engine/src/types/model.rs`
- `packages/file-types/json/aqc-package-json-engine/src/runtime/merge.rs`
- `packages/file-types/json/aqc-package-json-engine/src/runtime/reconcile.rs`
- `packages/file-types/json/aqc-package-json-engine/tests/contract.rs`

## Next Steps

- Commit the pnpm workspace YAML engine.
- Commit existing TOML engines coordinated onto file-engine-core 0.7.
