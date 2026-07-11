# Summary

Completed AQC adoption by making the newly enforced Clippy configuration pass in every workspace. Added shared I/O boundary bans, explicit generic-source lint decisions, and one narrow public-API waiver.

# Decisions made

- Apply the same forbidden filesystem, environment, and process APIs to every AQC workspace; existing boundary modules consume the expected exceptions.
- Allow `type_complexity` where public generic engine signatures require it.
- Allow long/nested reconciliation source shape only in the Cargo engine.
- Preserve the borrowed `RustToolchainProfile::as_str` API and waive only its lint-level finding.

# Key files for context

- `packages/**/clippy.toml`
- affected `Cargo.toml` lint tables
- `packages/file-types/toml/aqc-rust-toolchain-toml-engine/shakrs.json`

# Next steps

- Re-run the AQC Specular contract.
- Complete Shackles fixtures, adoption, release, and cross-repository verification.
