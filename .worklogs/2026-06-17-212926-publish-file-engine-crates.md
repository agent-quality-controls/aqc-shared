Summary
- Made `aqc-file-engine-core`, `aqc-cargo-toml-engine`, and `aqc-clippy-toml-engine` publishable.
- Added crate-local README and crates.io metadata for those crates.
- Added missing release metadata and README files for the already-public shared crates so local g3rs release checks have the expected package metadata.

Decisions made
- `aqc-file-engine-core` must publish first because both TOML engines depend on it.
- `aqc-cargo-toml-engine` and `aqc-clippy-toml-engine` keep local `path` dependencies plus `version = "0.1.0"` so local development still resolves through the workspace while published manifests can resolve through crates.io.
- Existing published crates `aqc-filetree`, `aqc-fs-utils`, and `aqc-git-helpers` were not version-bumped in this round; crates.io already has `0.1.0`, so the metadata changes apply to the repository and future releases.
- Broad code-size, import-count, and test-message g3rs code findings are not fixed in this publishing round because they require behavior-preserving module splits and test cleanup across existing engine code.

Key files for context
- `packages/aqc-file-engine-core/Cargo.toml`
- `packages/aqc-file-engine-core/README.md`
- `packages/file-types/toml/aqc-cargo-toml-engine/Cargo.toml`
- `packages/file-types/toml/aqc-cargo-toml-engine/README.md`
- `packages/file-types/toml/aqc-clippy-toml-engine/Cargo.toml`
- `packages/file-types/toml/aqc-clippy-toml-engine/README.md`
- `packages/aqc-filetree/Cargo.toml`
- `packages/aqc-fs-utils/Cargo.toml`
- `packages/aqc-git-helpers/Cargo.toml`

Verification
- `cargo publish --dry-run --allow-dirty --manifest-path packages/aqc-file-engine-core/Cargo.toml`
- `cargo publish --dry-run --allow-dirty --manifest-path packages/aqc-filetree/Cargo.toml`
- `cargo publish --dry-run --allow-dirty --manifest-path packages/aqc-fs-utils/Cargo.toml`
- `cargo publish --dry-run --allow-dirty --manifest-path packages/aqc-git-helpers/Cargo.toml`
- `cargo test -p aqc-file-engine-core -p aqc-cargo-toml-engine -p aqc-clippy-toml-engine`

Next steps
- Publish `aqc-file-engine-core` first.
- After crates.io index propagation, publish `aqc-cargo-toml-engine` and `aqc-clippy-toml-engine`.
- Then switch Guardrails publishable adapter crates to version-only dependencies on these shared crates and publish `g3rs-core` before the adapters.
