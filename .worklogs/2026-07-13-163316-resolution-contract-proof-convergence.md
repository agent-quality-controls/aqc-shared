# Resolution Contract Proof Convergence

## Summary

Closed the final adversarial proof gaps for exact/combined attribution and downstream resolved-root immutability.

## Decisions Made

- Asserted complete contributor sequences for Cargo and Clippy required, exact-only, and combined glob conflicts.
- Added external temporary consumer compilation that must reject both field mutation and struct construction for every resolved engine root.
- Kept runtime source and published versions unchanged because the reviewed implementation behavior was already correct.

## Key Files For Context

- `packages/file-types/toml/aqc-cargo-toml-engine/tests/dependency_globs.rs`
- `packages/file-types/toml/aqc-cargo-toml-engine/tests/dependency_identity.rs`
- `packages/file-types/toml/aqc-clippy-toml-engine/tests/merge.rs`
- `specs/verifiers/verify_resolution_contract_cleanup.py`

## Next Steps

- Obtain the final no-finding adversarial review, remove completed throwaway specs, and push.
