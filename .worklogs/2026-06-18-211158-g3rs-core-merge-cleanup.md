## Summary

Split `aqc-file-engine-core` merge machinery so old g3rs no longer rejects the core merge file or the workspace clippy gate at the core package boundary.

## Decisions Made

- Replaced `src/merge.rs` with a `src/merge/` module tree.
- Kept `merge/mod.rs` facade-only because g3rs requires `mod.rs` files to contain only module declarations and re-exports.
- Moved merge types, traits, and aliases into `merge/model.rs`.
- Moved item, list, and scalar composition logic into `merge/items.rs`, `merge/lists.rs`, and `merge/scalar.rs`.
- Added named type aliases for repeated provenance and requirement tuple shapes instead of allowing clippy `type_complexity`.
- Added explicit `Debug` bounds to merge identity associated types because the derived `Debug` contract on resolved requirement containers already required those identities to be debuggable.
- Renamed finding-level rendered contributor tuples to `RenderedContributors`.

## Verification

- `cargo fmt -p aqc-file-engine-core`
- `cargo test -p aqc-file-engine-core`
- `cargo clippy -p aqc-file-engine-core --all-targets --all-features -- -D warnings`
- `g3rs validate workspace --path .` still fails on the remaining Cargo/Clippy TOML engine findings, but the previous `aqc-file-engine-core/src/merge.rs` blocking errors are gone.

## Key Files For Context

- `packages/aqc-file-engine-core/src/merge/mod.rs`
- `packages/aqc-file-engine-core/src/merge/model.rs`
- `packages/aqc-file-engine-core/src/merge/items.rs`
- `packages/aqc-file-engine-core/src/merge/lists.rs`
- `packages/aqc-file-engine-core/src/merge/scalar.rs`
- `packages/aqc-file-engine-core/src/finding.rs`

## Next Steps

Continue with `aqc-cargo-toml-engine`:

- Reduce import counts in `src/lib.rs`, `src/reconcile/dependencies.rs`, and `src/reconcile/workspace_fields.rs`.
- Split large Cargo requirement/reconcile files.
- Fix public named-field findings for resolved Cargo requirement structs.
- Improve weak test `expect` messages and split oversized Cargo merge tests.
