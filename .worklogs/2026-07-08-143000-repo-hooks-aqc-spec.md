# Repo Hooks AQC Spec

## Summary

Added the AQC Specular contract for the repo hooks engine/core side.

The spec covers multi-file engine output, text file requirements, Git hooks requirements, AQC-only dependency boundaries, and text engine behavior tests.

## Decisions Made

- AQC checks live in the AQC repo because Specular paths are repo-relative.
- Text files and snippets must reuse `ItemRequirements` and `ResolvedItemRequirements`.
- Executable state remains a scalar assertion, not a separate assertion type.
- AQC crates must not name Shackles, Shakrs, policies, adapters, or command execution.

## Key Files For Context

- `specs/2026-07-08-104701-repo-hooks-aqc.spec.json`
- `specs/2026-07-08-104701-repo-hooks-aqc.spec.coverage.md`
- `specs/verifiers/verify_repo_hooks_aqc.py`
- `packages/aqc-file-engine-core/src/merge/model.rs`

## Verification

- `specular lint specs/2026-07-08-104701-repo-hooks-aqc.spec.json` passed.
- `specular verify specs/2026-07-08-104701-repo-hooks-aqc.spec.json` failed as expected because the AQC engine/core work is not implemented yet.

## Next Steps

- Implement multi-file engine output in `aqc-file-engine-core`.
- Add `aqc-text-file-engine`.
- Add `aqc-git-hooks-engine`.
