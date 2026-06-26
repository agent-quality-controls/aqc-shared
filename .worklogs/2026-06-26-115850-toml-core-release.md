# TOML core release

## Summary

Prepared the AQC shared side of the upstream duplication cleanup for release.
The change introduces `aqc-toml-engine-core`, keeps file-format-independent
merge primitives in `aqc-file-engine-core`, and migrates Cargo, Clippy, and
rustfmt TOML engines to shared TOML mechanics.

## Decisions made

- `aqc-file-engine-core` owns neutral merge and requirement primitives,
  including provenance-aware conflict construction and version tuple parsing.
- `aqc-toml-engine-core` owns reusable TOML parsing, table lookup, scalar
  matching/editing, list reconciliation, list-shape reporting, and finding
  helpers.
- Concrete TOML engines keep domain rules: Cargo dependency identity and
  workspace inheritance, Clippy ordered MSRV/threshold behavior, and rustfmt
  setting legality.
- Package includes for public core crates carry `tests/**`, so package dry-runs
  verify the same public contract tests as local runs.
- No compatibility aliases were kept for deleted local helpers.

## Key files for context

- `packages/aqc-file-engine-core/src/merge/scalar.rs`
- `packages/aqc-file-engine-core/src/version.rs`
- `packages/file-types/toml/aqc-toml-engine-core/src/lib.rs`
- `packages/file-types/toml/aqc-toml-engine-core/src/scalar.rs`
- `packages/file-types/toml/aqc-toml-engine-core/src/list.rs`
- `packages/file-types/toml/aqc-cargo-toml-engine/src/reconcile/package_lints.rs`
- `packages/file-types/toml/aqc-rustfmt-toml-engine/src/reconcile/settings/ignore.rs`

## Verification

- `cargo test --manifest-path Cargo.toml --workspace`
- `cargo package --manifest-path packages/aqc-file-engine-core/Cargo.toml --allow-dirty`
- `git diff --check`

## Next steps

- Publish `aqc-file-engine-core 0.3.2` first.
- Publish `aqc-toml-engine-core 0.3.2` after crates.io sees file-engine-core.
- Publish Cargo, Clippy, and rustfmt TOML engines after TOML core is available.
