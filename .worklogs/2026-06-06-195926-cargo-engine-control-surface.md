# Cargo engine control surface — built to spec (step 1)

## Summary
Rebuilt `aqc-cargo-toml-engine` to the reviewed control-surface spec
(`guardrail3/.plans/g3v2-architecture/2026-06-06-174328-cargo-engine-control-surface.md`):
the engine now exposes complete, locked control over `Cargo.toml` (17 targets),
every assertion classed writable / check-only in code and proven by a contract
catalogue. Core gained the shared primitives.

## What landed
- **Core (`aqc-file-engine-core`)**: `ConfigScalar { Str, Int, Bool }`, `Msg`,
  `FromEmpty { Writes, ChecksOnly }` + `FromEmptyClass::on_empty`, the
  `contract` module (`check_from_empty` — the two laws: writable writes once
  then settles clean; check-only writes nothing and never converges), and
  merge-vocabulary growth: `merge_map_by` (projection compare — how messages
  stay out of agreement), `union_first_wins`, `union_string_lists/sets`,
  `keyed_entries_eq`, `KeyedEntries`.
- **Engine requirement types** (`src/requirement/` module dir, one file per
  target family): PackageFieldAssertion (ConfigScalar + Msg + AtLeastVersion +
  ListExcludes + InheritsWorkspace), WorkspaceFieldAssertion (new),
  ManifestSection + SectionPresenceAssertion (new; `on_empty_in` is
  section-dependent), LintsInheritAssertion re-derived (Equals/Present/Absent),
  DependencyScope{kind,target} + 12-field DependencySpec (partial match, source
  rule) + entry aliases, FeatureSetAssertion + Msg, ProfileAssertion struct
  (fields + package_overrides + build_override; toml_edit::Value leak removed),
  TargetFieldAssertion/TargetTableAssertion (new), patch. The three set-style
  Resolve impls + their unions live in ONE macro (`impl_set_resolve!`) with the
  projection deciding semantic agreement; `impl_keyed_entries_eq!` for the
  message-insensitive equality.
- **Reconcile**: dispatch destructures the requirement exhaustively (no `..` —
  a new field fails compilation until wired) and enforces the
  inline-lints-vs-inherit exclusivity as a SchemaError; new modules
  workspace_fields, section_presence, target_tables ([[bin]] etc. keyed by
  name), patch; dependencies handle cfg-target scopes, the source rule, the
  string-shorthand write form, workspace-deps `optional` SchemaError; all
  apply paths create tables lazily so check-only never mutates.
- **Tests**: `tests/contract.rs` (31 tests — every variant of every assertion
  enum through the core harness, declared class vs actual behavior, plus the
  special rules) and `tests/merge.rs` updated (incl. dependency same-name
  conflict; message-agreement folded into `identical`).

## Verification
- `cargo clippy --workspace --all-targets`: clean.
- `cargo test --workspace`: all pass (31 contract + 4 merge + others).
- `cargo fmt --all -- --check`: clean. `cargo dupes check`: 7.5% exact (PASS).
- Both g3 manifest verifiers (engine + adapter) and the reconciliation +
  clippy-slice manifests: ALL LAYERS PASS (run from guardrail3).

## Notes / context for next sessions
- A background agent did roughly half the reconcile rework before dying on a
  server error; this session finished its in-flight state (its util.rs helper
  layout was kept).
- The `on_empty` method name (spec said `from_empty`) avoids clippy's
  `from_*-takes-no-self` convention; spec + manifest were amended in sync.
- The clippy-toml engine still uses its own embedded value types; adopting
  `ConfigScalar` there is a named later pass.
