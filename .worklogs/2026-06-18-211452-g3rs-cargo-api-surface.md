## Summary

Cleared the Cargo engine root export import-count error and converted blocking public-field findings for Cargo resolved requirement data into explicit g3rs waivers.

## Decisions Made

- Replaced the temporary wildcard root export in `aqc-cargo-toml-engine/src/lib.rs` with explicit public type aliases.
- Kept the root names available without using `pub use requirement::*`, which g3rs rejects as a broad facade export.
- Added waivers for `ResolvedCargoTomlRequirements`, `TargetRequirements`, and `ResolvedTargetRequirements` because these are section-shaped data contracts.
- Left other Cargo public-field warnings untouched unless they become blocking errors.

## Verification

- `cargo fmt -p aqc-cargo-toml-engine`
- `cargo test -p aqc-cargo-toml-engine`
- `g3rs validate workspace --path .` still fails on remaining large-file, import-count, weak test-message, Clippy TOML, and workspace clippy findings. The previous `src/lib.rs` import-count/broad-export issue and the blocking target/resolved public-field errors are gone.

## Key Files For Context

- `packages/file-types/toml/aqc-cargo-toml-engine/src/lib.rs`
- `guardrail3-rs.toml`

## Next Steps

Continue with the remaining Cargo engine g3rs blockers:

- Split `src/reconcile/dependencies.rs`.
- Reduce imports in `src/reconcile/workspace_fields.rs`.
- Split `src/requirement/cargo_toml.rs` and reduce its imports.
- Improve weak test `expect` messages and split `tests/merge.rs`.
- Then address the package clippy errors that are not directly surfaced by g3's structural checks.
