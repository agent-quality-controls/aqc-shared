# Plan: g3rs workspace cleanup

## Goal

Make `g3rs validate workspace --path .` pass, or reduce it through clean,
behavior-preserving commits until the remaining failures are isolated to one
explicitly scoped refactor.

## Current State

`g3rs validate repo --path .` passes.

`g3rs validate workspace --path .` fails on:

- large files and import-count findings in existing Cargo and Clippy TOML
  engines
- weak test `expect(...)` messages in Cargo and Clippy tests
- public named-field findings on requirement/fact structs
- `aqc-rust-syntax` not using a facade-only `lib.rs`
- `syn` and `proc-macro2` missing from `allowed_deps`
- release workflow/config files missing
- cargo clippy workspace gate failure

## Approach

1. Fix new crate structural findings first:
   - move `aqc-rust-syntax` implementation out of `src/lib.rs`
   - keep `src/lib.rs` as facade-only
   - add `syn` and `proc-macro2` to `allowed_deps`
   - add justified waivers for syntax fact structs if the public data shape is
     the intended API
2. Fix release infrastructure findings:
   - add minimal `release-plz.toml`
   - add minimal `cliff.toml`
   - add GitHub workflow entries with `cargo publish --dry-run`,
     `release-plz`, and `CARGO_REGISTRY_TOKEN`
3. Fix recently added rustfmt engine findings:
   - move non-facade code out of `reconcile/mod.rs`
   - decide whether Rustfmt requirement structs are data containers needing
     waivers or should become private-field APIs
4. Fix test-message findings mechanically:
   - replace weak `"utf8"`, `"msrv"`, `"equals mismatch"`, and similar
     messages with descriptive failure text
5. Fix import-count findings:
   - split long use lists or route grouped imports through local modules
   - do this file-by-file with tests after each package
6. Fix large-file findings:
   - split `merge.rs`, Cargo dependency reconcile, Cargo requirement model,
     Clippy requirement model, and oversized tests into focused sibling modules
   - preserve public exports and test behavior
7. Re-run:
   - `cargo fmt --all --check`
   - package tests for each touched crate
   - `cargo clippy --workspace --all-targets --all-features -- -D warnings`
   - `g3rs validate workspace --path .`

## Key Decisions

- Use waivers only where the named public fields are the intended data
  contract, matching existing project convention in `guardrail3-rs.toml`.
- Prefer moving implementation into sibling modules over suppressing
  facade-only architecture findings.
- Avoid changing serialized/file-engine behavior while splitting modules.
- Do not rewrite published public APIs unless g3rs has no cleaner local
  alternative.

## Files To Modify

- `guardrail3-rs.toml`
- `release-plz.toml`
- `cliff.toml`
- `.github/workflows/**`
- `packages/source/rust/aqc-rust-syntax/src/**`
- `packages/file-types/toml/aqc-rustfmt-toml-engine/src/**`
- `packages/file-types/toml/aqc-cargo-toml-engine/src/**`
- `packages/file-types/toml/aqc-cargo-toml-engine/tests/**`
- `packages/file-types/toml/aqc-clippy-toml-engine/src/**`
- `packages/file-types/toml/aqc-clippy-toml-engine/tests/**`
- `packages/aqc-file-engine-core/src/**`
