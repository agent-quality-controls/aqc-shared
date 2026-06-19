Summary

Implemented the shared engine side of scalar assertion unification. `aqc-file-engine-core` now owns the generic scalar assertion vocabulary and merge behavior, and Cargo, Clippy, and Rustfmt TOML engines now use it instead of local scalar assertion enums.

Decisions made

- Added `ScalarAssertion<T>`, `ScalarOperation`, and `ScalarValue` to core.
- Added `DottedVersion` as a generic dotted-numeric ordered value, without putting Rust, Cargo, or MSRV concepts into core.
- Kept Cargo product wrappers where Cargo fields can be scalar, list, ordered version, or workspace inheritance.
- Removed scalar-only local names such as `ProfileFieldAssertion`, `MsrvAssertion`, `NumericAssertion`, `BoolAssertion`, `StringAssertion`, `RustfmtScalarAssertion`, and `ResolvedRustfmtScalarAssertion`.
- Kept field/domain legality inside each engine: Cargo field names, Clippy scalar families, and Rustfmt setting kinds validate which scalar operations are accepted.
- Fixed Cargo `OrderedVersion(AtMost|Range)` to fail as field-legality `scalar-operation-unsupported` before generic composition.

Key files for context

- `packages/aqc-file-engine-core/src/merge/model.rs`
- `packages/aqc-file-engine-core/src/merge/scalar.rs`
- `packages/aqc-file-engine-core/src/types.rs`
- `packages/aqc-file-engine-core/tests/scalar_assertion.rs`
- `packages/file-types/toml/aqc-cargo-toml-engine/src/requirement/package.rs`
- `packages/file-types/toml/aqc-clippy-toml-engine/src/requirement/model.rs`
- `packages/file-types/toml/aqc-clippy-toml-engine/src/requirement/merge.rs`
- `packages/file-types/toml/aqc-rustfmt-toml-engine/src/requirement/mod.rs`
- `packages/file-types/toml/aqc-rustfmt-toml-engine/src/requirement/settings.rs`
- `packages/file-types/toml/aqc-clippy-toml-engine/tests/scalars.rs`

Verification

- `cargo fmt -p aqc-cargo-toml-engine -p aqc-clippy-toml-engine -p aqc-rustfmt-toml-engine -p aqc-file-engine-core` passed.
- `cargo test -p aqc-file-engine-core -p aqc-cargo-toml-engine -p aqc-clippy-toml-engine -p aqc-rustfmt-toml-engine --quiet` passed.
- Guardrail3 adapter, policy, runner, and Specular gates also passed against these shared changes.

Next steps

- Future engines should model scalar requirements with `ScalarAssertion<T>` and implement field-specific legality in the engine, not by creating local scalar verb enums.
- Product-shaped file concepts should remain engine-owned wrappers and should not be moved into core.
