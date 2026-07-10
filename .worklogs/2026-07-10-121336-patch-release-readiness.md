# Patch Release Readiness

## Summary

Prepared all publishable AQC crates for their patch releases. Package archives now include complete dual-license terms, manifests and lockfiles use the patch chain, and Cargo-deny matrices prohibit reverse and same-layer dependencies where applicable.

## Decisions Made

- Publish core and TOML engine crates as patch releases without changing public behavior.
- Publish `aqc-rust-syntax` as 0.3.3 because 0.3.2 was never published.
- Use one complete `LICENSE` payload in every package archive.
- Keep dependency architecture enforcement downstream in Shackles while every AQC workspace retains its own Cargo-deny gate.

## Key Files

- `packages/aqc-file-engine-core/Cargo.toml`
- `packages/file-types/toml/aqc-toml-engine-core/Cargo.toml`
- `packages/file-types/text/aqc-text-engine-core/Cargo.toml`
- `packages/file-types/toml/*/Cargo.toml`
- `packages/*/deny.toml`

## Next Steps

- Commit and push the release manifests and license payloads.
- Publish utilities and core crates before dependent engines.
- Verify each published version from crates.io.
