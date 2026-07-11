# Exact Text And TOML Cores

## Summary

Migrated reusable text and TOML reconciliation to the core exact-item API and prepared their breaking releases.

## Decisions Made

- TOML arrays and arrays of tables reconcile exact item collections through shared core state.
- Text contained-item exactness remains unsupported and reports invalid requirements without treating exact members as required content.
- Registry dependencies use `aqc-file-engine-core 0.5.0`; no path dependencies were added.

## Key Files For Context

- `packages/file-types/text/aqc-text-engine-core/src/reconcile.rs`
- `packages/file-types/toml/aqc-toml-engine-core/src/items/`

## Next Steps

- Publish both cores.
- Verify and release concrete TOML engines against them.
