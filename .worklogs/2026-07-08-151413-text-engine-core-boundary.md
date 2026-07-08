# Text Engine Core Boundary

## Summary

Replaced the reusable text package shape from a file engine to a core/helper crate so `aqc-git-hooks-engine` no longer depends on another file engine.

## Decisions Made

- Renamed `aqc-text-file-engine` to `aqc-text-engine-core`.
- Removed the `TextFileEngine` erased engine wrapper and `EngineRequirement` implementation from text requirements.
- Kept text requirement, merge, and reconcile mechanics reusable through `aqc-text-engine-core`.
- Updated `aqc-git-hooks-engine` to own the file-engine boundary and call the text helper directly.

## Key Files

- `packages/file-types/text/aqc-text-engine-core`
- `packages/file-types/git/aqc-git-hooks-engine`
- `specs/2026-07-08-104701-repo-hooks-aqc.spec.json`
- `specs/verifiers/verify_repo_hooks_aqc.py`

## Next Steps

- Publish AQC crates in dependency order.
- Update Shackles manifests and specs to use `aqc-text-engine-core` through `aqc-git-hooks-engine`.
