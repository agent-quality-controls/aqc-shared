# AQC Independent Workspaces

## Summary

- Converted AQC Rust packages from one root Cargo workspace to independent package workspaces.
- Added package-local `deny.toml` and `Cargo.lock` files, removed root Cargo workspace files, and updated the release workflow to iterate package manifests.

## Decisions Made

- Removed root `Cargo.toml`, root `Cargo.lock`, and root `deny.toml` because the requested architecture has no root Rust workspace.
- Replaced workspace-inherited package metadata with explicit package metadata in every AQC package manifest.
- Removed cross-package path dependencies from AQC package manifests; dependencies are versioned crate dependencies.
- Added per-package deny files that forbid wrong AQC directions and forbid any product crate dependency from AQC.
- Kept package-local `Cargo.lock` files because each package is now an independent workspace.
- Updated `.github/workflows/release.yml` so publish dry-runs and release-plz use package manifests instead of root `--workspace`.

## Key Files For Context

- `.github/workflows/release.yml`
- `packages/aqc-file-engine-core/Cargo.toml`
- `packages/file-types/toml/aqc-toml-engine-core/Cargo.toml`
- `packages/file-types/toml/aqc-cargo-toml-engine/Cargo.toml`
- `packages/**/deny.toml`

## Verification

- `cargo check --manifest-path <aqc package>/Cargo.toml --all-targets` passed for every AQC package.
- `cargo deny --manifest-path <aqc package>/Cargo.toml check` passed for every AQC package.
- Cross-repo Specular verification from the Shackles repo passed.

## Open Issue

- `g3rs validate repo --path aqc-shared` still fails on the old topology marker-pair rule because AQC no longer has a root Cargo workspace and package workspaces do not each have `guardrail3-rs.toml`.
- Fixing that cleanly means either migrating g3rs config to every independent package or changing the g3rs topology rule. I did not hide it with a non-working waiver.

