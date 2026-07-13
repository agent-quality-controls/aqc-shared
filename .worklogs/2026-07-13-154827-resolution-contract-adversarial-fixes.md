# Resolution Contract Adversarial Fixes

## Summary

Closed two release-blocking gaps found by the final Task 1 adversarial review: exact-only Cargo dependencies now participate in identity invariants, and Cargo/Clippy engines require the core patch that exports `asserted_items`.

## Decisions Made

- Reused `aqc_file_engine_core::asserted_items` for invalid identity and duplicate file-key checks so required and exact collections follow one rule.
- Added exact-only regression cases for missing identity and two packages sharing one TOML key.
- Released Cargo and Clippy engine patch versions with `aqc-file-engine-core = "0.6.3"` as the minimum compatible API.
- Strengthened the Specular verifier to reject an engine minimum below the API generation it uses.
- Brought the Cargo engine's Clippy configuration into compliance with the strict dogfood policy reported by the commit hook.

## Key Files For Context

- `packages/file-types/toml/aqc-cargo-toml-engine/src/requirement/cargo_toml/conflicts.rs`
- `packages/file-types/toml/aqc-cargo-toml-engine/tests/dependency_identity.rs`
- `specs/verifiers/verify_resolution_contract_cleanup.py`

## Next Steps

- Publish Cargo and Clippy engine 0.6.2.
- Update Shackles registry locks and application patch releases, then repeat the final adversarial review.
