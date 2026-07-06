# Toolchain Production Release

## Summary

Published the AQC crates needed for the Rust toolchain vertical to resolve through crates.io.
`aqc-file-engine-core` is now `0.3.3`, and `aqc-rust-toolchain-toml-engine` is published as `0.3.3`.

## Decisions Made

- Bumped `aqc-file-engine-core` to `0.3.3` because `0.3.2` was already published and `DottedVersion` now exposes serde support.
- Kept `aqc-rust-toolchain-toml-engine` at `0.3.3` because it was a first publish.
- Added an explicit `serde` test use in `scalar_assertion.rs` so the package-level `unused_crate_dependencies` gate accepts the test dependency.
- Rejected local patch dependencies for production verification; the published engine resolved `aqc-file-engine-core = "0.3.3"` from crates.io.

## Key Files

- `packages/aqc-file-engine-core/Cargo.toml`
- `packages/aqc-file-engine-core/tests/scalar_assertion.rs`
- `packages/file-types/toml/aqc-rust-toolchain-toml-engine/Cargo.toml`
- `packages/file-types/toml/aqc-rust-toolchain-toml-engine/tests/behavior.rs`

## Verification

- `cargo test --manifest-path packages/aqc-file-engine-core/Cargo.toml --all-targets`
- `cargo test --manifest-path packages/file-types/toml/aqc-rust-toolchain-toml-engine/Cargo.toml --all-targets`
- `cargo publish --dry-run` and `cargo publish` for both published crates
- `cargo info aqc-file-engine-core`
- `cargo info aqc-rust-toolchain-toml-engine`

## Next Steps

- Keep AQC engine versions published before dependent Shackles crates are released.
- Do not add local path dependencies for published package resolution.
