# pnpm Workspace YAML Engine

## Summary

Added the pathless `aqc-pnpm-workspace-yaml-engine` for resolving and reconciling pnpm workspace configuration bytes. The engine uses shared file-engine requirements and YAML-engine-core syntax, merge, and write mechanics.

## Decisions Made

- Modeled direct and inherited YAML mapping values without filesystem or pnpm command execution.
- Preserved YAML merge sources and wrote deterministic direct overrides only when requirements authorize a value.
- Split reconciliation into facade, application, and support modules to keep public and internal roles clear.
- Attached selectors and provenance to forbidden package findings and conflicts.

## Key Files For Context

- `packages/file-types/yaml/aqc-pnpm-workspace-yaml-engine/src/types/model.rs`
- `packages/file-types/yaml/aqc-pnpm-workspace-yaml-engine/src/types/merge.rs`
- `packages/file-types/yaml/aqc-pnpm-workspace-yaml-engine/src/runtime/reconcile/`
- `packages/file-types/yaml/aqc-pnpm-workspace-yaml-engine/tests/engine_requirement.rs`

## Next Steps

- Commit existing TOML engines coordinated onto file-engine-core 0.7.
- Commit the AQC fixture, Specular, and release inventory integration.
