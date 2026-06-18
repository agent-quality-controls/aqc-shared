## Summary

Replaced weak Cargo engine test `expect(...)` messages reported by old g3rs with specific failure messages.

## Decisions Made

- Replaced `"utf8"` with `"engine output should be valid UTF-8 TOML"` in Cargo engine tests.
- Replaced `"exact mismatch"` with `"expected an exact-list mismatch finding"`.
- Kept this as a mechanical test-only change with no production behavior changes.

## Verification

- `rg 'expect\("utf8"\)|expect\("exact mismatch"\)' packages/file-types/toml/aqc-cargo-toml-engine/tests -n`
- `cargo fmt -p aqc-cargo-toml-engine`
- `cargo test -p aqc-cargo-toml-engine`
- `g3rs validate workspace --path .` still fails on remaining large-file, import-count, Clippy TOML, and workspace clippy findings. The Cargo weak test-message findings are gone.

## Key Files For Context

- `packages/file-types/toml/aqc-cargo-toml-engine/tests/contract.rs`
- `packages/file-types/toml/aqc-cargo-toml-engine/tests/contract_tables.rs`
- `packages/file-types/toml/aqc-cargo-toml-engine/tests/merge.rs`

## Next Steps

- Reduce Cargo test import counts.
- Split `tests/merge.rs`.
- Split `src/reconcile/dependencies.rs` and `src/requirement/cargo_toml.rs`.
- Reduce imports in `src/reconcile/workspace_fields.rs`.
