Summary:
- Made the three AQC shared crates used by Specular publishable and published `aqc-filetree`, `aqc-fs-utils`, and `aqc-git-helpers` v0.1.0 to crates.io.
- Added license files and corrected the workspace repository URL for the public `agent-quality-controls/aqc-shared` repo.

Decisions made:
- Removed `publish = false` only from the three crates Specular needs.
- Left other shared crates unpublished.
- Kept the workspace license as `MIT OR Apache-2.0` and added both license files.

Key files for context:
- `Cargo.toml`
- `CONVENTIONS.md`
- `packages/aqc-filetree/Cargo.toml`
- `packages/aqc-fs-utils/Cargo.toml`
- `packages/aqc-git-helpers/Cargo.toml`

Next steps:
- Configure crates.io Trusted Publishing for the three shared crates after this commit is pushed.
