# Git Hooks File Engine Surface

## Summary

Removed stale Git config value types from `aqc-git-hooks-engine`.
The engine now exposes only Git hook file requirements and delegates reusable text mechanics to `aqc-text-engine-core`.

## Decisions Made

- Kept `core.hooksPath` out of AQC file engines because it is Git local config, not stable file bytes.
- Removed `GitHooksPath` and `GitHooksValueError` instead of preserving unused public API.
- Updated the AQC Specular contract to forbid those stale types in the Git hooks engine.

## Key Files For Context

- `packages/file-types/git/aqc-git-hooks-engine/src/requirement/model.rs`
- `packages/file-types/git/aqc-git-hooks-engine/src/lib.rs`
- `specs/2026-07-08-104701-repo-hooks-aqc.spec.json`
- `specs/2026-07-08-104701-repo-hooks-aqc.spec.coverage.md`

## Verification

- `cargo test --locked` in `packages/file-types/git/aqc-git-hooks-engine`
- `specular lint specs/2026-07-08-104701-repo-hooks-aqc.spec.json`
- `specular verify specs/2026-07-08-104701-repo-hooks-aqc.spec.json`

## Next Steps

- Publish `aqc-git-hooks-engine` 0.1.1.
- Update Shackles Git hooks adapter and policy to depend on the cleaned engine surface.
