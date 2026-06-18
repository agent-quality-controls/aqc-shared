## Summary

Started the old g3rs cleanup by fixing the new `aqc-rust-syntax` package shape and adding release configuration files required by the release checks.

## Decisions Made

- Split `aqc-rust-syntax` into a facade plus `model` and `parser` modules so `src/lib.rs` stays a small public API surface.
- Added an `api` feature and gated the public syntax exports behind it because g3rs requires shared-crate facade exports to be feature-gated.
- Added `proc-macro2` and `syn` to the dependency allowlist because the syntax crate uses `syn` parsing and `proc-macro2` line spans directly.
- Added plain-data waivers for `RustFileSyntax`, `RustEnumDecl`, and `RustSyntaxError`; those types are parser facts, not behavioral API objects.
- Added `release-plz.toml`, `cliff.toml`, and a release workflow with publish dry-run and release-plz jobs, then adjusted them to match the old g3rs release rules.

## Verification

- `cargo fmt -p aqc-rust-syntax`
- `cargo test -p aqc-rust-syntax`
- `cargo clippy -p aqc-rust-syntax --all-targets -- -D warnings`
- `g3rs validate workspace --path .` still fails on remaining pre-existing workspace findings and on publish dry-run refusing uncommitted package changes.

## Key Files For Context

- `guardrail3-rs.toml`
- `packages/source/rust/aqc-rust-syntax/Cargo.toml`
- `packages/source/rust/aqc-rust-syntax/src/lib.rs`
- `packages/source/rust/aqc-rust-syntax/src/model.rs`
- `packages/source/rust/aqc-rust-syntax/src/parser.rs`
- `release-plz.toml`
- `cliff.toml`
- `.github/workflows/release.yml`

## Next Steps

Continue clearing the old g3rs workspace findings:

- Split `packages/aqc-file-engine-core/src/merge.rs`.
- Reduce import counts and split large files in `aqc-cargo-toml-engine`.
- Fix or waive public named fields in resolved requirement data structures based on whether each type is transport data or an API object.
- Improve weak test `expect` messages in cargo and clippy engine tests.
- Split `aqc-clippy-toml-engine/src/requirement.rs`.
- Rerun `g3rs validate workspace --path .` after committing this batch so publish dry-run sees a clean tree.
