# Worklog: universal item requirement cleanup

## Summary

Cleaned the file-engine requirement API by removing stale aliases and using
explicit structs or plain `String` fields in the public surface.

This completes the alias-removal pass required by the guardrail3 universal
item-requirement architecture.

## Decisions Made

- Removed `DependencyEntry` instead of keeping a compatibility alias.
- Replaced `FeatureEntry = BTreeSet<String>` with
  `FeatureMembers { members: BTreeSet<String> }`.
- Replaced `LintEntry = (String, Option<i64>)` with
  `LintSetting { level: String, priority: Option<i64> }`.
- Removed `Msg`, `PolicyId`, and `ConflictContributors` aliases; public
  structures now use `String` and `Vec<(String, String)>` directly.
- Kept dependency package-identity matching in the Cargo reconciler:
  `file_key: None` plus `DependencySpec.package` validates by effective
  package, while explicit `file_key` validates by Cargo left key.

## Verification

- `cargo fmt`
- `cargo test -p aqc-file-engine-core -p aqc-cargo-toml-engine -p aqc-clippy-toml-engine`
- `cargo msrv verify --rust-version 1.85 -- cargo check --locked` through the
  guardrail3 cargo adapter dependency path
- Adversarial review found dead dependency overlap code in cargo merge; it was
  removed, then tests were rerun.

## Key Files For Context

- `packages/aqc-file-engine-core/src/merge.rs`
- `packages/aqc-file-engine-core/src/types.rs`
- `packages/aqc-file-engine-core/src/finding.rs`
- `packages/file-types/toml/aqc-cargo-toml-engine/src/requirement/dependencies.rs`
- `packages/file-types/toml/aqc-cargo-toml-engine/src/reconcile/dependencies.rs`
- `packages/file-types/toml/aqc-cargo-toml-engine/src/requirement/features.rs`
- `packages/file-types/toml/aqc-cargo-toml-engine/src/requirement/lints.rs`
- `packages/file-types/toml/aqc-cargo-toml-engine/tests/merge.rs`

## Next Steps

- Consumers should use `ItemRequirements<DependencyRequirement>` for
  dependency checks and `String` for messages.
- Do not add aliases back for old public names.
