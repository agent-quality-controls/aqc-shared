# Rustfmt Ignore Forbidden Globs

## Summary

Added forbidden glob support for `rustfmt.toml` `ignore = [...]` entries in
`aqc-rustfmt-toml-engine`. The engine now uses the shared
`ForbiddenGlobRequirements` merge model and `globset`, matching Cargo and
Clippy engines.

## Decisions Made

- Glob support applies only to Rustfmt `ignore` list values.
- Scalar settings, setting names, and `skip_macro_invocations` do not get glob
  support.
- `RustfmtIgnorePathGlob` is the file-engine type for forbidden `ignore`
  path globs.
- Conflicts between required `ignore` values and forbidden ignore globs block
  the conflicting glob during reconciliation.
- Existing matching `ignore` values are removed and reported as
  `absent (path glob)`.
- Invalid glob syntax reports `Finding::InvalidRequirements`.

## Key Files

- `.plans/2026-06-18-213431-rustfmt-ignore-globs.md`
- `.plans/2026-06-18-213431-rustfmt-ignore-globs.md.spec.json`
- `packages/file-types/toml/aqc-rustfmt-toml-engine/src/requirement.rs`
- `packages/file-types/toml/aqc-rustfmt-toml-engine/src/reconcile/settings.rs`
- `packages/file-types/toml/aqc-rustfmt-toml-engine/tests/merge.rs`
- `packages/file-types/toml/aqc-rustfmt-toml-engine/tests/reconcile.rs`

## Verification

- `cargo fmt --package aqc-rustfmt-toml-engine --check`
- `cargo test -p aqc-rustfmt-toml-engine`
- `specular lint .plans/2026-06-18-213431-rustfmt-ignore-globs.md.spec.json`
- `specular verify .plans/2026-06-18-213431-rustfmt-ignore-globs.md.spec.json`
- `git diff --check -- packages/file-types/toml/aqc-rustfmt-toml-engine .plans/2026-06-18-213431-rustfmt-ignore-globs.md .plans/2026-06-18-213431-rustfmt-ignore-globs.md.spec.json .plans/2026-06-18-213431-rustfmt-ignore-globs.md.spec.coverage.md Cargo.lock`

## Residual Risk

- `cargo clippy -p aqc-rustfmt-toml-engine --all-targets -- -D warnings`
  still fails on broader existing package lint debt, mostly private docs and
  type complexity in the Rustfmt engine. New similar-name and shadowing issues
  from this slice were fixed.

## Next Steps

- If Rustfmt engine clippy-clean status is required, handle it as a separate
  cleanup of the package structure and private-doc lint strategy.
