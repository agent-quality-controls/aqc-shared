# Cargo Workspace Lints Section

## Summary

Added `ManifestSection::WorkspaceLints` to `aqc-cargo-toml-engine` so requirements can validate and initialize `[workspace.lints]`.
This supports Cargo manifests that use `[lints] workspace = true`, which Cargo rejects when the workspace lint table is missing.

## Decisions Made

- Modeled `[workspace.lints]` as a Cargo manifest section instead of making Shackles or the runner synthesize TOML directly.
- Reused `aqc-toml-engine-core` table helpers to create the nested table.
- Kept `[package]` present as check-only because the engine still cannot invent package name or target semantics.
- Bumped `aqc-cargo-toml-engine` to `0.3.4`.

## Key Files For Context

- `packages/file-types/toml/aqc-cargo-toml-engine/src/requirement/sections.rs`
- `packages/file-types/toml/aqc-cargo-toml-engine/src/reconcile/section_presence.rs`
- `packages/file-types/toml/aqc-cargo-toml-engine/tests/contract.rs`
- `packages/file-types/toml/aqc-cargo-toml-engine/Cargo.toml`

## Verification

- `cargo fmt --manifest-path packages/file-types/toml/aqc-cargo-toml-engine/Cargo.toml --check`
- `cargo test --manifest-path packages/file-types/toml/aqc-cargo-toml-engine/Cargo.toml --all-targets`
- `cargo deny --manifest-path packages/file-types/toml/aqc-cargo-toml-engine/Cargo.toml check`
- `cargo package --manifest-path packages/file-types/toml/aqc-cargo-toml-engine/Cargo.toml --allow-dirty`
- Shackles spec `cargo-workspace-lints-contract`

## Next Steps

- Publish `aqc-cargo-toml-engine v0.3.4` before publishing Shackles crates that depend on it.
