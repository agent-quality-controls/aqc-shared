Summary:
- Split `aqc-rustfmt-toml-engine/src/reconcile/settings.rs` into focused settings modules.
- Added targeted waivers for rustfmt public data-shape structs.
- Cleared rustfmt settings file-size and public-field error-level g3rs findings.

Decisions made:
- Split settings reconciliation into dispatch, scalar, list, forbidden-ignore-glob, closed-settings, and TOML helper modules instead of waiving the large file.
- Waived rustfmt requirement aggregate structs and plain glob/conflict records because they are public API data contracts.

Verification:
- `cargo fmt -p aqc-rustfmt-toml-engine`
- `cargo test -p aqc-rustfmt-toml-engine`
- `g3rs validate workspace --path . 2>&1 | rg 'aqc-rustfmt-toml-engine|^\[Error\]'`

Remaining issues:
- `aqc-rustfmt-toml-engine/tests/reconcile.rs` still has a file-size g3rs error.

Key files for context:
- `packages/file-types/toml/aqc-rustfmt-toml-engine/src/reconcile/settings/mod.rs`
- `packages/file-types/toml/aqc-rustfmt-toml-engine/src/reconcile/settings/apply.rs`
- `packages/file-types/toml/aqc-rustfmt-toml-engine/src/reconcile/settings/scalar.rs`
- `packages/file-types/toml/aqc-rustfmt-toml-engine/src/reconcile/settings/list.rs`
- `packages/file-types/toml/aqc-rustfmt-toml-engine/src/reconcile/settings/ignore.rs`
- `packages/file-types/toml/aqc-rustfmt-toml-engine/src/reconcile/settings/closed.rs`
- `packages/file-types/toml/aqc-rustfmt-toml-engine/src/reconcile/settings/toml_io.rs`
- `guardrail3-rs.toml`

Next steps:
- Split `aqc-rustfmt-toml-engine/tests/reconcile.rs` by behavior group while preserving assertions.
