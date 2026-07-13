# Summary

Aligned existing AQC workspace G3 and Shackles configuration with the established published-library Cargo, Clippy, and cargo-deny source shape. The change records precise waivers for intentional public facades, manifest lint allowances, and stricter dependency auditing.

# Decisions Made

- Kept AQC's stricter `deny.toml` settings instead of weakening them to the G3 application baseline.
- Added selector-specific waivers only for manifest entries and source-shaped deny settings present in each workspace.
- Kept `aqc-file-engine-core::merge` as a pure public facade and documented its import-count exception.
- Added the standard `std::process::abort` prohibition required by the strict Clippy policy.

# Key Files For Context

- `packages/aqc-file-engine-core/guardrail3-rs.toml`
- `packages/aqc-filetree/guardrail3-rs.toml`
- `packages/aqc-fs-utils/guardrail3-rs.toml`
- `packages/aqc-git-helpers/guardrail3-rs.toml`
- `packages/source/rust/aqc-rust-syntax/guardrail3-rs.toml`

# Next Steps

- Commit the PNPM AQC engine vertical after its Specular and Fixture3 gates pass.
