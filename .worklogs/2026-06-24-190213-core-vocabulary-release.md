Summary
- Moved the shared file-engine vocabulary to `aqc-file-engine-core` ownership and cleaned concrete TOML engine roots so they no longer re-export core vocabulary.
- Renamed item reconciliation vocabulary from banned to forbidden through core, Cargo, and Clippy engine call sites.
- Published the AQC file-engine crates at 0.3.1 for downstream Guardrail and Specular use.
- The earlier 0.3.0 publish was superseded because final clippy/module-shape cleanup changed the committed source after that publish.

Decisions made
- `aqc-file-engine-core` is the source of shared vocabulary such as `ItemRequirements`, `ResolvedItemRequirements`, and forbidden item resolution types.
- Concrete engines keep file-specific terms only where they model file syntax or domain concepts.
- Clippy config concepts use `disallowed` where that is the file/domain term, while the shared reconciliation concept remains `forbidden`.
- No deprecated aliases or backward-compatible banned names were kept.

Key files for context
- `packages/aqc-file-engine-core/src/merge/items.rs`
- `packages/aqc-file-engine-core/src/merge/model.rs`
- `packages/file-types/toml/aqc-cargo-toml-engine/src/reconcile/dependencies/apply.rs`
- `packages/file-types/toml/aqc-cargo-toml-engine/src/reconcile/dependencies/removals.rs`
- `packages/file-types/toml/aqc-clippy-toml-engine/src/requirement/disallowed.rs`
- `packages/file-types/toml/aqc-clippy-toml-engine/src/reconcile/disallowed.rs`

Verification
- `cargo fmt --all` passed.
- `cargo test -p aqc-file-engine-core -p aqc-cargo-toml-engine -p aqc-clippy-toml-engine -p aqc-rustfmt-toml-engine --quiet` passed.
- Package dry-runs passed for all four AQC crates.
- Published crates: `aqc-file-engine-core`, `aqc-cargo-toml-engine`, `aqc-clippy-toml-engine`, and `aqc-rustfmt-toml-engine` at 0.3.1.

Residual risks
- The rename is intentionally source-breaking for consumers using old banned identifiers.
- Published versions cannot be replaced on crates.io, so any follow-up fix needs a new semver version.

Next steps
- Keep new shared file-engine vocabulary in core only.
- Add future file-engine vocabulary to core first when it is shared across engines, not to concrete engine roots.
