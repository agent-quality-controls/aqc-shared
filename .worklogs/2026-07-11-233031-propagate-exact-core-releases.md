# Summary

Advanced every AQC file engine that consumes the corrected exact-item cores and refreshed its lockfile. Updated the Cargo dependency conflict regression to the core contract that merge conflicts stop reconciliation.

# Decisions made

- Require `aqc-file-engine-core` 0.5.2 from every dependent engine.
- Require `aqc-toml-engine-core` 0.5.1 from every TOML engine.
- Publish patch releases for text, Cargo, Clippy, Deny, Rustfmt, and rust-toolchain engines.
- Expect one attributed merge conflict and no reconciliation findings after a dependency merge conflict.

# Key files for context

- `packages/file-types/text/aqc-text-engine-core/Cargo.toml`
- `packages/file-types/toml/*/Cargo.toml`
- `packages/file-types/toml/aqc-cargo-toml-engine/tests/dependency_identity.rs`
- `.plans/2026-07-11-213527-create-only-init-and-exact-items.md` in Shackles

# Next steps

- Publish the six dependent engine releases.
- Update Shackles consumers and release the CLI.
- Finish Specular, Fixture3, adoption, and adversarial gates.
