# Summary

Added `aqc-rust-toolchain-toml-engine` for `rust-toolchain.toml` validation and init. The engine exposes typed scalar/list/closed settings through the same AQC requirement model used by other TOML engines.

During adversarial review, fixed invalid init behavior where a `path` requirement could report a conflict with channel-based requirements while still writing an invalid file.

# Decisions Made

- Engine owns only `rust-toolchain.toml` syntax and rustup setting rules.
- It does not know Shakrs, policies, adapters, Cargo, or MSRV.
- `channel`, `path`, and `profile` are scalar settings.
- `components` and `targets` are list settings.
- `path` with a value blocks channel-based settings.
- `path = Absent` is compatible with `channel` because it removes the incompatible file field.
- Relative path requirements are invalid and are not written.
- Unknown settings are allowed when open and rejected when `closed_settings` is present.

# Key Files

- `packages/file-types/toml/aqc-rust-toolchain-toml-engine/src/lib.rs`
- `packages/file-types/toml/aqc-rust-toolchain-toml-engine/src/requirement/model.rs`
- `packages/file-types/toml/aqc-rust-toolchain-toml-engine/src/requirement/settings.rs`
- `packages/file-types/toml/aqc-rust-toolchain-toml-engine/src/reconcile/settings.rs`
- `packages/file-types/toml/aqc-rust-toolchain-toml-engine/tests/reconcile.rs`
- `specs/2026-07-03-212312-rust-toolchain-toml-engine.spec.json`
- `specs/verifiers/verify_rust_toolchain_engine.py`

# Verification

- `cargo test --manifest-path packages/file-types/toml/aqc-rust-toolchain-toml-engine/Cargo.toml --all-targets`
- `cargo deny --manifest-path packages/file-types/toml/aqc-rust-toolchain-toml-engine/Cargo.toml check`
- `specular lint specs/2026-07-03-212312-rust-toolchain-toml-engine.spec.json`
- `specular verify specs/2026-07-03-212312-rust-toolchain-toml-engine.spec.json`

# Next Steps

- Publish this crate before publishing Shakrs crates that depend on it.
- Regenerate downstream lockfiles after crates.io publication so local patched lock entries become registry entries.
