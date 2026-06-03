# Requirement reconciliation: the engine merge phase (aqc-shared side)

## Summary
Added the disk-independent merge phase to the file engines. Many policies/adapters
now write one file: their `MergedAssertion` contributions reach one engine, which
unions them per key and resolves each to one value or a `PolicyConflict`. Conflicts
are computed before any disk read; the disk diff (`Mismatch`) stays a separate phase.

## Decisions
- **`merge.rs` (core) owns *how* to merge; each engine owns *its fields*.** The seam
  is the `Resolve` trait. Added a `reason` field to `ConflictEntry`
  (`scalar-disagree` / `set-key-disagree` / `exact-mismatch`) set by the strategy
  that fired, so the engine can surface it on `Finding::PolicyConflict` without
  reconstructing it.
- **`Finding::PolicyConflict` reshaped** to per-key/per-policy:
  `{ subject, key, contributors: Vec<(PolicyId, String)>, reason }`. Always Error,
  field dropped, not waivable.
- **Cargo engine merge** (`requirement.rs`): `Resolve` impls per assertion (set/map
  variants union keys; scalar/exact must agree), `CargoTomlRequirement::merge` (union
  each field then `resolve_field`/`resolve_optional`), and a NEW
  `LintsInheritAssertion` + `lints_inherit` field + `reconcile/lints_inherit.rs` for
  the `[lints] workspace = <bool>` opt-in. Erased `Engine::reconcile` now does
  merge -> apply -> map conflicts; the `_`-arm `InternalError` is gone.
- **Message-insensitivity:** the policy-authored `message` is documentation, not a
  value, so same-value/different-message contributions agree. Cargo lint maps do this
  via a custom union; the clippy engine does it via hand-written `PartialEq`.
- **Objective merge (msrv/edition max) stays DEFERRED:** every scalar disagreement
  conflicts until a real composition case lands.
- Clippy engine (`aqc-clippy-toml-engine`) got the same treatment (Resolve + merge +
  erased reconcile), done by a delegated agent against the cargo engine as reference.

## Verification
- `cargo clippy --workspace --all-targets`: clean.
- `cargo test --workspace`: all pass. Merge probes:
  `aqc-cargo-toml-engine -- merge::` (disjoint/conflict/identical, 3 pass);
  `aqc-clippy-toml-engine -- merge::` (8 pass incl. ban/threshold union cases).

## Key files
- `packages/aqc-file-engine-core/src/merge.rs` — Resolve, ConflictEntry (+reason), strategies.
- `packages/aqc-file-engine-core/src/finding.rs` — PolicyConflict shape.
- `packages/file-types/toml/aqc-cargo-toml-engine/src/requirement.rs` — Resolve impls + merge + LintsInheritAssertion.
- `.../aqc-cargo-toml-engine/src/engine.rs`, `src/reconcile/lints_inherit.rs`, `tests/merge.rs`.
- `.../aqc-clippy-toml-engine/src/{requirement,engine}.rs`, `tests/merge.rs`.

## Known / deferred
- Conflict contributor rendering uses `{:?}` (Debug), which embeds the message for
  some variants. Cosmetic, display-only; parity across both engines.
- Objective-merge whitelist (msrv/edition take-the-max) deferred.

## Next steps
- The guardrail3 side (clippy adapter strip, runner, cargo policy/adapter, fixtures)
  landed in the guardrail3 repo on `development`. See that worklog.
