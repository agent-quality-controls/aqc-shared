# Taxonomy unification — core + engines side

## Summary
Engine-side half of the taxonomy pass (guardrail3
`.plans/g3v2-architecture/2026-06-07-115121-taxonomy-unification.md`; the noun
register is `taxonomy.md` there). Renames and one structural change, no
behavior change except where named.

## What changed
- `Finding`: `Mismatch.path`/`UnwritableRequiredKey.path` → `key`;
  `PolicyConflict` → `ConflictingRequirements`; `SchemaError` →
  `InvalidRequirements { key, message, contributors }` — its documented
  "file violates its own schema" meaning had zero emitters and never will
  (the engine does not re-validate the tool's schema); the real meaning is a
  jointly-unwritable requirement set. Contributors ADDED (the old emitters
  named no policies). Preference order documented at the definition: types →
  one-key either/or modeling → this variant for relational constraints only.
- `MergedAssertion` (wrapper struct), `Contributions`, `AssertionMap`,
  `KeyedEntries` (aliases) all deleted: collected assertions are plainly
  `Vec<(Provenance, A)>` everywhere, with module-level
  `#[expect(clippy::type_complexity, reason = ...)]` declaring the relaxation
  openly instead of dodging it via aliases.
- `FromEmpty` → `OnEmpty`, `FromEmptyClass` → `OnEmptyClass`.
- **Cargo engine: `[lints]` is ONE either/or key.** `lints` + `lints_inherit`
  fields replaced by `package_lints` with `PackageLintsAssertion::Inherit(bool,
  Msg) | Inline(BTreeMap<tool, LintLevelsAssertion>)` (cargo's own rule says a
  manifest is one or the other). The dispatch exclusivity SchemaError is
  DELETED — mixed inherit/inline now surfaces from the merge as an ordinary
  `ConflictingRequirements` at `[lints]` naming both policies (new merge test
  pins this). `LintsInheritAssertion` is gone (Present/Absent had no surviving
  use). Reconcile module `lints_inherit.rs` → `package_lints.rs` (Inherit
  writes the opt-in; Inline re-pairs per tool and reuses the lint-table
  reconcile).
- The two remaining `InvalidRequirements` emitters (InheritsWorkspace under
  `[workspace.package]`; `optional` in `[workspace.dependencies]`) now carry
  contributors. The word "contribution" is retired from prose.

## Verification
clippy workspace clean; all tests (17+14 contract, 5 merge incl. the new
inherit-vs-inline conflict, clippy-engine suites); fmt; dupes 8.2% (pass);
g3rs validate pre-commit gate; all five guardrail3 manifests PASS afterwards.

## Next steps
- Scope-narrowing the two remaining InvalidRequirements emitters into types is
  a recorded separate judgment (near-duplicate types vs compile-time catch).
