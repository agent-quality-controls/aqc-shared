# Rustfmt Ignore Forbidden Globs

## Goal

Support forbidden glob checks for `rustfmt.toml` `ignore = [...]` entries in
the shared Rustfmt TOML engine.

## Approach

- Add `RustfmtIgnorePathGlob`.
- Add `forbidden_ignore_path_globs` to `RustfmtTomlRequirements`.
- Resolve the field through `resolve_forbidden_globs`.
- Reconcile globs only against `RustfmtListSetting::Ignore` values.
- Report and remove matching ignored paths from `ignore`.
- Report invalid glob syntax as invalid requirements.
- Keep exact list requirements as the writer surface.

## Decisions

- Do not add glob support to Rustfmt scalar settings.
- Do not add glob support to Rustfmt setting names.
- Do not add glob support to `skip_macro_invocations`.
- Use `globset`, matching Cargo and Clippy engines.

## Files To Modify

- `packages/file-types/toml/aqc-rustfmt-toml-engine/Cargo.toml`
- `packages/file-types/toml/aqc-rustfmt-toml-engine/src/requirement.rs`
- `packages/file-types/toml/aqc-rustfmt-toml-engine/src/reconcile/settings.rs`
- `packages/file-types/toml/aqc-rustfmt-toml-engine/src/lib.rs`
- `packages/file-types/toml/aqc-rustfmt-toml-engine/tests/engine_requirement.rs`
- `packages/file-types/toml/aqc-rustfmt-toml-engine/tests/merge.rs`
- `packages/file-types/toml/aqc-rustfmt-toml-engine/tests/reconcile.rs`

## Verification

- `cargo test -p aqc-rustfmt-toml-engine`
- `cargo fmt --package aqc-rustfmt-toml-engine --check`
- `specular verify .plans/2026-06-18-213431-rustfmt-ignore-globs.md.spec.json`
