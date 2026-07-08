# Summary

Implemented the AQC side of the repo hooks vertical: multi-file engine output in core, a reusable text file engine, and a Git hooks engine that delegates text reconciliation.

# Decisions Made

- `aqc-file-engine-core` now exposes `EngineFileState`, `EngineFileOutput`, and multi-file `EngineOutput`.
- Existing TOML engines were updated to the new erased `Engine` API while keeping typed single-file reconciliation.
- Single-file engine findings remain in aggregate `EngineOutput.findings`; multi-file engines can attach findings to individual files.
- `aqc-text-file-engine` owns exact text bytes, required snippets, and executable metadata.
- `aqc-git-hooks-engine` owns Git hook path vocabulary and delegates file bytes to the text engine.
- Versioned path dependencies are used inside AQC workspaces so unpublished local crates compile together before release.

# Key Files

- `packages/aqc-file-engine-core/src/engine.rs`
- `packages/aqc-file-engine-core/src/types.rs`
- `packages/file-types/text/aqc-text-file-engine`
- `packages/file-types/git/aqc-git-hooks-engine`
- `specs/2026-07-08-104701-repo-hooks-aqc.spec.json`

# Verification

- `cargo test --locked` passed for changed AQC core, TOML core, all updated TOML engines, text engine, and Git hooks engine.
- `specular verify specs/2026-07-08-104701-repo-hooks-aqc.spec.json` passed.

# Next Steps

- Build the Shackles repo-level hooks policy, adapter, runner scope, CLI commands, and fixtures against these AQC engines.
