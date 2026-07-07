# Deny TOML Engine Release

## Summary

Added `aqc-deny-toml-engine` and reusable TOML array item reconciliation in `aqc-toml-engine-core`.
Migrated Clippy disallowed arrays to the shared TOML item helper so deny did not introduce a second implementation of the same array item mechanics.

## Decisions Made

- `aqc-toml-engine-core` now owns generic TOML array and array-of-tables item reconciliation.
- Clippy keeps forbidden path-glob behavior local because glob matching is Clippy domain logic, not TOML item mechanics.
- `aqc-deny-toml-engine` models fixed `deny.toml` fields directly with core `ScalarAssertion`, `ListRequirements`, and `ItemRequirements`.
- `aqc-clippy-toml-engine` is bumped to `0.3.5` because `0.3.3` is already published and the source changed.
- `aqc-toml-engine-core` is bumped to `0.3.5` and depends on `aqc-file-engine-core = "0.3.3"` so public item-helper types do not split across core versions.
- `aqc-deny-toml-engine` reconciliation and value logic is split into smaller internal modules instead of waiving large-file hook failures.
- The new deny workspace includes the same release baseline files as current Rust workspaces: `LICENSE`, `rust-toolchain.toml`, `rustfmt.toml`, `clippy.toml`, `deny.toml`, and `guardrail3-rs.toml`.
- `aqc-clippy-toml-engine/deny.toml` now forbids direct dependencies on `aqc-deny-toml-engine` and `aqc-rust-toolchain-toml-engine`.

## Key Files For Context

- `.plans/2026-07-07-103006-deny-toml-engine.md`
- `specs/2026-07-07-103006-deny-toml-engine.spec.json`
- `specs/verifiers/verify_deny_toml_engine.py`
- `packages/file-types/toml/aqc-toml-engine-core/src/items/`
- `packages/file-types/toml/aqc-clippy-toml-engine/src/reconcile/disallowed.rs`
- `packages/file-types/toml/aqc-deny-toml-engine/src/requirement/model.rs`
- `packages/file-types/toml/aqc-deny-toml-engine/src/requirement/merge_helpers.rs`
- `packages/file-types/toml/aqc-deny-toml-engine/src/reconcile/items.rs`
- `packages/file-types/toml/aqc-deny-toml-engine/src/reconcile/scalar_apply.rs`

## Verification

- `cargo test --manifest-path packages/file-types/toml/aqc-toml-engine-core/Cargo.toml --all-targets`
- `cargo test --manifest-path packages/file-types/toml/aqc-clippy-toml-engine/Cargo.toml --all-targets`
- `cargo test --manifest-path packages/file-types/toml/aqc-deny-toml-engine/Cargo.toml --all-targets`
- `specular lint specs/2026-07-07-103006-deny-toml-engine.spec.json`
- `specular verify specs/2026-07-07-103006-deny-toml-engine.spec.json`
- `cargo package` for TOML core, Clippy, and deny engine

Current spec stamp:

- spec: `767cedb80584e1adbc4e1e46c39f5131b157a1ed09f88efc21fe65a5138eb8bf`
- verifier: `83672f3a88bf4ff68f3b5ec6c0a8d49b0632884a08cb3655fed7d6501a8f8fb5`

## Next Steps

Publish in this order:

1. `aqc-toml-engine-core 0.3.5` - already published during commit unblocking
2. `aqc-clippy-toml-engine 0.3.4`
3. `aqc-deny-toml-engine 0.1.0`

After publishing, verify from crates.io in a temporary consumer crate with no path dependencies.
