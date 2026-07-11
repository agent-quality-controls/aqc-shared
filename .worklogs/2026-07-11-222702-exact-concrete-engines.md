# Exact Concrete Engines

## Summary

Migrated Cargo, Clippy, Deny, Rustfmt, and Rust toolchain engines to exact collection vocabulary and released dependency pins.

## Decisions Made

- Cargo models `[lints.<tool>]` table identities separately from their contents.
- Inline lint requirements imply the corresponding table identity.
- Exact collection findings preserve exact messages and provenance.
- Deny, Rustfmt, and toolchain scalar-key closure is named `exact_settings` with no legacy aliases.

## Key Files For Context

- `packages/file-types/toml/aqc-cargo-toml-engine/src/requirement/cargo_toml/model.rs`
- `packages/file-types/toml/aqc-cargo-toml-engine/src/reconcile/package_lint_tables.rs`
- `packages/file-types/toml/aqc-*/src/requirement/model.rs`
- `specs/create-only-init-and-exact-items.spec.json`

## Next Steps

- Publish concrete engines in parallel.
- Update Shackles adapters and policies to the released APIs.
