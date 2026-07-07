# Deny Production Shape Repair

## Summary

Fixed `aqc-deny-toml-engine` so `licenses.confidence-threshold` is an ordered cargo-deny float and `advisories.maximum-db-staleness` rejects the known invalid non-cargo-deny duration form.
Published `aqc-deny-toml-engine v0.1.1`.

## Decisions Made

- Kept `ScalarAssertion::AtLeast` in `aqc-file-engine-core`; no new core assertion was needed.
- Changed `DenyConfidenceThreshold` semantics in the deny engine value type because that type owns the field's ordering and TOML representation.
- Stored confidence thresholds with canonical text and an integer ordering key so merge uses deterministic ordering without float equality.
- Made deny reconciliation parse and write confidence thresholds as TOML floats, matching cargo-deny.
- Required `DenyDuration` strings to start with `P`, preventing the known rejected `90d` value from entering requirements.
- Added tests for `AtLeast(0.8)` accepting `0.9`, repairing `0.7`, and writing `confidence-threshold = 0.8`.

## Key Files For Context

- `.plans/2026-07-07-141120-deny-production-shape-repair.md`
- `packages/file-types/toml/aqc-deny-toml-engine/src/requirement/value.rs`
- `packages/file-types/toml/aqc-deny-toml-engine/src/requirement/value/value_impls/core.rs`
- `packages/file-types/toml/aqc-deny-toml-engine/src/reconcile/scalar_value.rs`
- `packages/file-types/toml/aqc-deny-toml-engine/tests/reconcile.rs`
- `packages/file-types/toml/aqc-deny-toml-engine/tests/merge.rs`
- `specs/verifiers/verify_deny_toml_engine.py`

## Verification

- `cargo fmt --manifest-path packages/file-types/toml/aqc-deny-toml-engine/Cargo.toml`
- `cargo test --manifest-path packages/file-types/toml/aqc-deny-toml-engine/Cargo.toml --all-targets`
- `cargo deny --manifest-path packages/file-types/toml/aqc-deny-toml-engine/Cargo.toml check`
- `specular lint specs/2026-07-07-103006-deny-toml-engine.spec.json`
- `specular verify specs/2026-07-07-103006-deny-toml-engine.spec.json`
- `cargo package --manifest-path packages/file-types/toml/aqc-deny-toml-engine/Cargo.toml --allow-dirty`
- `cargo publish --manifest-path packages/file-types/toml/aqc-deny-toml-engine/Cargo.toml --allow-dirty`

## Next Steps

- Update Shackles deny policy to emit `ScalarAssertion::AtLeast(DenyConfidenceThreshold::new("0.8"), ...)`.
- Update Shackles deny policy to use `DenyDuration::new("P90D")`.
- Verify installed `shakrs` output with `cargo deny check`.
