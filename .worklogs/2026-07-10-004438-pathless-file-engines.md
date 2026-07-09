# Pathless File Engines

## Summary

Updated AQC file-engine infrastructure so engines no longer know file paths, report subjects, executable bits, or filesystem roots. Engines now reconcile one byte stream from grouped requirements and return expected bytes plus findings.

## Decisions Made

- Removed path-aware engine output/state types from `aqc-file-engine-core`.
- Kept grouped erased reconciliation through `Engine::reconcile(current_bytes, reqs)`.
- Updated TOML engines to the pathless engine contract.
- Reworked `aqc-text-engine-core` into a generic text byte engine for exact contents and required snippets.
- Deleted the Git hooks file engine; hooks are represented by Shackles adapter requirements lowered into generic text requirements.
- Added source-shape and public-contract tests to prevent engines from regaining path/root responsibilities.

## Key Files For Context

- `packages/aqc-file-engine-core/src/engine.rs`
- `packages/aqc-file-engine-core/src/types.rs`
- `packages/aqc-file-engine-core/src/finding.rs`
- `packages/aqc-file-engine-core/tests/public_contract.rs`
- `packages/aqc-file-engine-core/tests/architecture.rs`
- `packages/file-types/text/aqc-text-engine-core/src/engine.rs`
- `packages/file-types/text/aqc-text-engine-core/src/requirement/model.rs`
- `packages/file-types/text/aqc-text-engine-core/src/reconcile.rs`
- `packages/file-types/toml/*/src/engine.rs`

## Verification

- Verified through Shackles `./scripts/check-workspaces.sh`, which runs all AQC workspace deny, clippy, and test gates with local patches.
- Verified by Shackles Specular specs:
  - `specs/2026-07-09-191854-engine-target-routing.spec.json`
  - `specs/2026-07-09-210317-persistent-architecture-guardrails.spec.json`

## Next Steps

- Publish AQC crates before publishing Shackles crates that depend on the new versions.
- Keep any future file-format engines pathless; file placement remains runner/application responsibility.
