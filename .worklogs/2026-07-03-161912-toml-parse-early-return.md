# TOML Parse Early Return

## Summary

Cargo, Clippy, and Rustfmt TOML engines now stop reconciliation immediately
after parse errors. Malformed TOML returns parse findings with empty expected
bytes and does not cascade into synthetic mismatch findings.

## Decisions Made

- The fix belongs inside each TOML file engine `reconcile` implementation.
- Engines return `EngineOutput { expected_bytes: Vec::new(), findings }` when
  parsing fails.
- Direct engine tests assert malformed TOML produces exactly one parse finding
  and no replacement bytes.

## Key Files

- `packages/file-types/toml/aqc-cargo-toml-engine/src/engine.rs`
- `packages/file-types/toml/aqc-cargo-toml-engine/tests/contract.rs`
- `packages/file-types/toml/aqc-clippy-toml-engine/src/engine.rs`
- `packages/file-types/toml/aqc-clippy-toml-engine/tests/scalars.rs`
- `packages/file-types/toml/aqc-rustfmt-toml-engine/src/engine.rs`
- `packages/file-types/toml/aqc-rustfmt-toml-engine/tests/reconcile_scalars.rs`

## Verification

- `cargo test --manifest-path packages/file-types/toml/aqc-cargo-toml-engine/Cargo.toml --all-targets`
- `cargo test --manifest-path packages/file-types/toml/aqc-clippy-toml-engine/Cargo.toml --all-targets`
- `cargo test --manifest-path packages/file-types/toml/aqc-rustfmt-toml-engine/Cargo.toml --all-targets`

## Next Steps

- Keep these tests in release gates so Specular's static coverage check is
  backed by runtime engine behavior.
