# Summary

Removed the obsolete root verifier that depended on a deleted Guardrail3 plan and asserted the pre-redesign AQC API. Added Specular enforcement that these duplicate architecture scripts remain absent.

# Decisions made

- Kept package-local Cargo, Cargo-deny, Shakrs, and current Specular gates as the maintained verification paths.
- Deleted the stale scripts instead of rewriting a second architecture manifest beside the Task 1 spec.
- Added every removed script to the Specular tree forbidden set.

# Key files for context

- `specs/resolution-contract-cleanup.spec.json`
- `scripts/`
- `packages/aqc-file-engine-core/guardrail3-rs.toml`

# Next steps

- Complete final cross-repository adversarial review.
- Push AQC after Shackles application verification completes.
