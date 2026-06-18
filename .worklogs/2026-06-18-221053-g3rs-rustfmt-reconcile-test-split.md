Summary:
- Split `aqc-rustfmt-toml-engine/tests/reconcile.rs` into scalar, list, and ignore/closed integration tests.
- Kept the existing assertions and small reconcile helpers duplicated in each focused test file.
- Cleared the rustfmt reconcile test file-size g3rs error.

Decisions made:
- Duplicated the small test helpers instead of using `#[path]`, `include!`, or a `mod.rs` helper because g3rs forbids those patterns.
- Kept each split test file self-contained to avoid shared-helper dead-code warnings under `-D warnings`.

Verification:
- `cargo fmt -p aqc-rustfmt-toml-engine`
- `cargo test -p aqc-rustfmt-toml-engine`
- `g3rs validate workspace --path . 2>&1` now reaches the cargo clippy gate; no rustfmt file-size g3rs errors remain.

Remaining issues:
- Workspace `g3rs validate` still fails because `cargo clippy --workspace --all-targets --all-features -- -D warnings` reports many lint errors across the cargo, clippy, and rustfmt TOML engines.

Key files for context:
- `packages/file-types/toml/aqc-rustfmt-toml-engine/tests/reconcile_scalars.rs`
- `packages/file-types/toml/aqc-rustfmt-toml-engine/tests/reconcile_lists.rs`
- `packages/file-types/toml/aqc-rustfmt-toml-engine/tests/reconcile_ignore_closed.rs`

Next steps:
- Triage the clippy gate failures by crate and fix or justify them in small batches.
