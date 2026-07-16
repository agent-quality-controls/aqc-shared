# Summary

Unified exact string-list difference calculation and desired-value construction in `aqc-file-engine-core`, then adopted it in JSON, TOML, Cargo TOML, Deny TOML, Rust toolchain TOML, Rustfmt TOML, and pnpm workspace YAML reconciliation. Exact-list findings now identify individual members and duplicate counts while reserving selectorless findings for absence, malformed shape, and pure order drift.

# Decisions made

- Core owns format-neutral multiset differences and application order for exact, contains, and excludes requirements.
- Format engines retain syntax-specific finding keys, parsing, shape diagnostics, and writes.
- JSON list and glob mechanics live in a private reconciliation submodule; the document reconciler only orchestrates paths and requirement families.
- Malformed Cargo list fields emit shape findings and still produce repaired expected bytes.
- Existing owned `String` TOML public signatures remain unchanged in the patch release; no alias or compatibility API was added.
- CSpell reverse dependency prohibitions were added to AQC Cargo Deny files without adding Shackles dependencies.
- Dependency-only workspace lockfiles were synchronized to the same AQC patch generation so isolated `cargo metadata --locked` gates remain reproducible.
- Adversarial review findings were fixed: target directories are excluded from verifier source discovery, nested engine workspaces remain covered, independent requirement attribution is tested, dependency-only locks permit only AQC patch-version changes, workspace architecture rules run in Specular, and the plan matches the public count API.

# Key files for context

- `.plans/2026-07-15-211236-exact-list-differences.md`
- `packages/aqc-file-engine-core/src/merge/lists.rs`
- `packages/aqc-file-engine-core/src/merge/model.rs`
- `packages/file-types/toml/aqc-toml-engine-core/src/lists.rs`
- `specs/exact-list-differences.spec.json`
- `fixtures/scripts/fixture3-exact-list-differences.py`

# Next steps

- Publish the bumped AQC patch releases before publishing downstream Shackles crates that require them.
- Keep future exact-list engines on the core difference and application APIs rather than adding format-local copies.
