# AQC g3rs Workspace Markers

## Goal

- Remove the repo-root guardrail3-rs.toml marker because aqc-shared has no root Cargo workspace.
- Add guardrail3-rs.toml beside each package-local Cargo.toml that declares [workspace].
- Preserve existing waivers by moving each waiver into the workspace that owns the subject file.

## Approach

- Delete root guardrail3-rs.toml.
- Add one local config per AQC package workspace.
- Keep paths local to each workspace, for example src/types.rs instead of packages/.../src/types.rs.
- Run g3rs validate repo from the aqc-shared root.
- Run g3rs validate workspace for each adopted package workspace.

## Files

- guardrail3-rs.toml
- packages/*/guardrail3-rs.toml
- packages/file-types/toml/*/guardrail3-rs.toml
- packages/source/rust/aqc-rust-syntax/guardrail3-rs.toml
