# AQC g3rs Workspace Markers

## Summary

- Removed the repo-root guardrail3-rs.toml marker because aqc-shared has no root Cargo workspace.
- Added guardrail3-rs.toml beside each package Cargo.toml that declares [workspace].
- Moved existing waivers into the owning workspace config with paths relative to that workspace.

## Decisions

- Kept dependency allowlists local to each package instead of copying the old repo-wide list everywhere.
- Disabled toolchain, fmt, clippy, deps, test, and release in these package-local configs for now.
- Reason: AQC currently keeps rust-toolchain.toml, rustfmt.toml, clippy.toml, and release files at the repo root, while old g3rs workspace validation expects those files beside each workspace marker.
- Kept cargo and code enabled so local dependency allowlists and source-shape checks still run.

## Verification

- g3rs validate repo
- g3rs validate workspace for all nine package workspaces

## Key Files

- packages/aqc-file-engine-core/guardrail3-rs.toml
- packages/aqc-filetree/guardrail3-rs.toml
- packages/aqc-fs-utils/guardrail3-rs.toml
- packages/aqc-git-helpers/guardrail3-rs.toml
- packages/file-types/toml/aqc-toml-engine-core/guardrail3-rs.toml
- packages/file-types/toml/aqc-cargo-toml-engine/guardrail3-rs.toml
- packages/file-types/toml/aqc-clippy-toml-engine/guardrail3-rs.toml
- packages/file-types/toml/aqc-rustfmt-toml-engine/guardrail3-rs.toml
- packages/source/rust/aqc-rust-syntax/guardrail3-rs.toml

## Next Steps

- Decide whether AQC should move toolchain/fmt/clippy/release config files into every independent workspace, or whether g3rs should support repo-shared files for multi-workspace repos.
