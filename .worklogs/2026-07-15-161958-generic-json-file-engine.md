# Generic JSON File Engine

## Summary

Added a reusable strict JSON file engine over existing AQC requirement and JSON CST primitives. Repaired the shared scalar finding-key contract and added approved behavior fixtures.

## Decisions

- Use RFC 6901 pointers for non-root finding keys and `$` for the root object.
- Keep typed tool vocabularies downstream; the generic engine supports scalar, string-list, forbidden-glob, and object-key requirements only.
- Treat empty collection requirements as no-ops and invalid globs as atomic requirement failures.
- Keep Package JSON and TSConfig finding identities unchanged by making the shared scalar key explicit.
- Make shared list/item conflict identities format-aware through core `FindingKey`; JSON uses RFC 6901 escaping while existing string callers retain dot keys.
- Stop object reconciliation after a blocked parent produces its one shape finding.
- Resolve JSON presence separately from value kind so closure rejects only descendants that must exist and direct required/absent disagreements fail during merge.
- Deduplicate required-list members before glob conflict detection and combine all matching-glob contributors into one escaped member-keyed conflict.
- Leave same-surface presence contradictions to core resolution and use JSON presence resolution only for cross-surface contradictions.
- Leave same-object child membership contradictions to core item resolution; JSON closure handles only requirements originating outside that parent object requirement.
- Suppress a kind conflict only when opposite presence requirements explain every incompatible contributor pair at that JSON path.
- Carry presence polarity on each kind occurrence instead of inferring requirement identity from non-unique policy provenance.
- Treat forbidden/nonconstructive kind pairs as compatible while retaining conflicts involving a required value.
- Attribute kind conflicts only to occurrences participating in unexplained kind pairs.
- Add core `push_rendered_conflict` and route all JSON custom conflicts through its deterministic contributor ordering.

## Verification

- `specular lint` and `specular verify` pass for spec SHA-256 `0b3c3a793edc0794598a4f105b3aad9a53a3595bf68bee393a7cdbdd036bc739` and verifier SHA-256 `c8d6ea26c0defdcc0e26ca447f52c889cf82d32f2313e4b3db3d69ee7bfd4b5b`.
- All three AQC Fixture3 suites match approved output.
- File-engine core, JSON core, generic JSON engine, Package JSON, TSConfig, and every affected resolver caller pass the spec's format, test, Clippy, deny, and package gates.
- Independent review confirmed conflict identity, attribution, deterministic ordering, caller coverage, and exact changed-file scope, then reported no findings.
- The commit hook caught an obsolete fixture-probe Guardrail3 schema; the probe now uses the repository's current workspace marker format.

## Key Files

- `.plans/2026-07-15-142457-generic-json-file-engine.md`
- `packages/file-types/json/aqc-json-file-engine/src/types/model.rs`
- `packages/file-types/json/aqc-json-file-engine/src/runtime/merge.rs`
- `packages/file-types/json/aqc-json-file-engine/src/runtime/reconcile.rs`
- `fixtures/probes/generic-json-file-engine/src/main.rs`
- `specs/generic-json-file-engine.spec.json`

## Next Steps

- Publish the coordinated AQC release before publishing downstream Shackles crates.
- Commits are split by dependency tier so each pre-commit Cargo gate resolves only its unpublished upstream crates through the approved local-source configuration.
