# Generic JSON File Engine

## Status

IMPLEMENTATION-READY. The initial adversarial review found ambiguous scalar
finding keys, contradictory object closure, non-atomic invalid-glob handling,
and incomplete collection behavior. The corrections below are part of the
frozen architecture.

## Goal

Add one reusable strict-JSON file engine for configuration files whose managed
surface is composed of scalar values, string lists, and object-key membership. Keep JSON syntax and
editing in AQC, reuse all requirement composition from
`aqc-file-engine-core`, and leave tool-specific closed vocabularies in typed
concrete engines.

## Evidence

- `aqc-json-engine-core` already owns lossless JSON/JSONC parsing, duplicate-key
  rejection, scalar lookup/editing, and format-specific parse options.
- `aqc-file-engine-core` already owns scalar, list, forbidden-glob,
  provenance, conflict, finding, and resolved-requirement primitives.
- `aqc-package-json-engine` has a typed Package JSON surface shared by pnpm,
  TSC, Prettier, and future adapters. It remains a concrete engine.
- `aqc-tsconfig-json-engine` has a typed TypeScript compiler-option vocabulary
  and JSONC dialect. It remains a concrete engine.
- Prettier, CSpell, and jscpd configuration can be expressed as scalar values
  and string lists addressed by JSON object paths.
- Syncpack cannot use this engine for `versionGroups`: array order and
  first-match semantics are part of the file meaning, so it requires a typed
  concrete engine when implemented.

## Business Behavior

- Strict JSON object input is required. Invalid syntax, duplicate keys,
  invalid UTF-8, and non-object roots preserve the supplied bytes and report a
  parse finding.
- Scalar, string-list, forbidden-glob, and object-key requirements compose
  across policies with complete provenance.
- Missing writable values initialize deterministically. Existing malformed
  containers and blocked parents are reported without replacement.
- Invalid requirement globs prevent every edit in the requirement set.
- Validation returns findings without writing. Init consumes expected bytes
  through runner create-only behavior; the engine performs no IO.

## Ownership And Flow

```text
JsonFileRequirements
  -> ResolvedJsonFileRequirements
  -> JsonFileEngine
  -> EngineOutput
```

- `aqc-file-engine-core` owns assertion algebra, resolution, provenance,
  conflicts, findings, lists, items, forbidden globs, and format-neutral
  finding-key composition.
- `aqc-json-engine-core` owns JSON CST parsing, lookup, and editing.
- `aqc-json-file-engine` owns strict-JSON structural requirements and their
  reconciliation. It knows no tool, policy, path, filename, or filesystem.

## Public Surface

New crate: `aqc-json-file-engine`.

```rust
pub struct JsonPath { /* root or object-key components */ }

impl JsonPath {
    pub fn root() -> Self;
    pub fn new(first: impl Into<String>) -> Self;
    pub fn child(self, component: impl Into<String>) -> Self;
pub fn components(&self) -> impl Iterator<Item = &str>;
    pub fn pointer(&self) -> String;
    pub fn selector(&self) -> String;
}

pub struct JsonStringGlob {
    pub glob: String,
}

pub struct JsonFileRequirements {
    pub scalar_values: BTreeMap<JsonPath, ScalarAssertion<ConfigScalar>>,
    pub string_lists: BTreeMap<JsonPath, ListRequirements>,
    pub forbidden_string_list_globs:
        BTreeMap<JsonPath, ForbiddenGlobRequirements<JsonStringGlob>>,
    pub object_keys: BTreeMap<JsonPath, ItemRequirements<KeyedItem<()>>>,
}

pub struct ResolvedJsonFileRequirements { /* private fields */ }

pub struct JsonFileEngine;
```

Shared core collection resolution accepts a `FindingKey` supplied by the
format engine. Existing string keys retain dot-separated child identities;
`JsonPath` produces escaped RFC 6901 child identities. Core therefore owns
collection composition without assuming one file format's path syntax.

Empty JSON object keys remain representable. `JsonPath::root()` addresses the
root object only. The crate re-exports only the core vocabulary required to construct
`JsonFileRequirements`. It does not alias or duplicate core types.
`ScalarValue` is part of that construction surface because downstream adapters
define typed scalar enums and implement the core trait through the engine
facade; adapters may not depend on file-engine core directly.

## Resolution

- Group every `JsonFileRequirements` by provenance.
- Resolve scalar assertions with core `resolve_map`; `ScalarAssertion<T>`
  already implements core `Resolve`.
- Resolve string lists with `resolve_list`.
- Resolve forbidden globs with `resolve_forbidden_globs`.
- A path appearing as both a scalar and a string list is a requirement
  conflict. Resolution returns conflicts and reconciliation does not run.
- A forbidden-list-glob requirement may share a path with string-list
  requirements because both address the same string array.
- A scalar path conflicts with a list/glob path at the same location.
- Any scalar/list leaf conflicts with every managed descendant path because a
  JSON value cannot be both a leaf and an object.
- Object-key requirements may be ancestors of managed descendants, but
  conflict with a scalar/list leaf at the same path.
- Presence semantics are resolved separately from value kinds. Object closure
  conflicts only with descendants that require presence; absent, excludes-only,
  and forbidden-glob-only descendants remain compatible. Required object keys
  conflict with a direct `Absent` requirement.
- Required or exact list members matched by a forbidden glob conflict before
  reconciliation. Each member emits one escaped member-keyed conflict with
  complete required and matching-glob attribution.
- Contributor order is deterministic through core merge functions.
- Resolved fields are private and exposed through borrowed getters.

## Reconciliation

- Parse as strict JSON: no comments, trailing commas, single quotes, loose
  keys, JSONC number extensions, invalid UTF-8, duplicate keys, or non-object
  roots.
- Missing bytes initialize an object and apply every resolved requirement.
- Existing valid bytes preserve unaddressed values and existing formatting.
- Scalar requirements use `reconcile_scalar_assertion`.
- String-list requirements require an array containing only strings.
- `contains`, `excludes`, and `exact` use core resolved list semantics.
- Forbidden globs match each string list member with the established glob
  matcher and emit one selector-specific finding per matching member.
- Wrong scalar, list, parent-object, or item shape emits a finding and does
  not replace unrelated existing data.
- Object-key requirements reuse core required/forbidden/exact item semantics.
  Forbidden and exact-extra keys are removed from expected bytes. A required
  key with no separate value requirement is reported as unwritable.
- An absent list is not created for excludes-only or forbidden-glob-only
  requirements. Contains appends in resolved key order. Exact controls complete
  order and exact-empty creates an empty array. Malformed arrays and blocked
  parents remain byte-identical.
- Exact-empty list and object creation emits a mismatch before producing
  expected bytes. Empty list, glob, and object requirement values are no-ops.
- Reconcile constructive descendant objects before ancestor object-key checks,
  preventing stale unwritable findings for keys created in the same pass.
- Findings use an RFC 6901 JSON Pointer as the unambiguous key and the final path component or
  offending list item as the selector. Root-object findings use `$` because
  RFC 6901 represents the root with an empty string, which is not a usable
  waiver key.
- Validation never changes input bytes.
- Reconciliation returns expected bytes only; it performs no IO and knows no
  path, filename, workspace, tool, policy, adapter, or runner concept.

## JSON Core Changes

Extend `aqc-json-engine-core::JsonObject` with reusable strict string-array
mechanics:

```rust
pub fn string_list(&self, path: &[&str]) -> Option<Vec<String>>;
pub fn value_is_array(&self, path: &[&str]) -> bool;
pub fn set_string_list(
    &mut self,
    path: &[&str],
    values: &[String],
    parent_action: NonObjectParentAction,
) -> bool;
pub fn set_object(
    &mut self,
    path: &[&str],
    parent_action: NonObjectParentAction,
) -> bool;
pub fn object_keys(&self, path: &[&str]) -> Option<Vec<String>>;
pub fn remove_object_key(&mut self, path: &[&str], key: &str) -> bool;
```

These methods know JSON CST operations only. They do not know requirement
types, tool settings, or policy meaning.

Repair the existing shared parent-write traversal at the same boundary:
replacing an intermediate masked scalar must discard metadata at the replaced
parent path, not only at the requested leaf path. Cover scalar and list writes.

Do not move Package JSON or TSConfig requirement roots into JSON core. Do not
make either concrete engine depend on `aqc-json-file-engine`.

## Package Boundaries

- `aqc-json-file-engine -> aqc-json-engine-core -> aqc-file-engine-core`
- `aqc-json-file-engine -> aqc-file-engine-core`
- no engine-to-engine dependency
- no filesystem, process, environment, network, or Shackles dependency
- no cross-workspace path dependency

## Files

Add the independent workspace under:

- `packages/file-types/json/aqc-json-file-engine/Cargo.toml`
- `packages/file-types/json/aqc-json-file-engine/Cargo.lock`
- `packages/file-types/json/aqc-json-file-engine/deny.toml`
- `packages/file-types/json/aqc-json-file-engine/LICENSE`
- `packages/file-types/json/aqc-json-file-engine/README.md`
- `packages/file-types/json/aqc-json-file-engine/src/lib.rs`
- `packages/file-types/json/aqc-json-file-engine/src/types/*`
- `packages/file-types/json/aqc-json-file-engine/src/runtime/*`
- `packages/file-types/json/aqc-json-file-engine/tests/*`

Modify JSON core for reusable CST mechanics and the explicit scalar finding-key
contract. Update only the scalar-reconciler call sites in Package JSON and
TSConfig so their existing finding identities remain unchanged. Add AQC
Specular files and the final worklog.

Modify file-engine core to add and export `FindingKey`, and make shared list
and item resolution use it for child conflict identities:

- `packages/aqc-file-engine-core/src/finding.rs`
- `packages/aqc-file-engine-core/src/lib.rs`
- `packages/aqc-file-engine-core/src/merge/items.rs`
- `packages/aqc-file-engine-core/src/merge/lists.rs`

Update release dependency gates in every existing independent workspace whose
`deny.toml` resolves the coordinated AQC generation:

- `aqc-file-engine-core`, `aqc-filetree`, `aqc-fs-utils`, `aqc-git-helpers`
- `aqc-json-engine-core`, `aqc-package-json-engine`, `aqc-tsconfig-json-engine`
- `aqc-text-file-engine`
- `aqc-cargo-toml-engine`, `aqc-clippy-toml-engine`, `aqc-deny-toml-engine`
- `aqc-rust-toolchain-toml-engine`, `aqc-rustfmt-toml-engine`, `aqc-toml-engine-core`
- `aqc-pnpm-workspace-yaml-engine`, `aqc-yaml-engine-core`
- `aqc-rust-syntax`

## Integration

- Publish `aqc-json-file-engine` before downstream adapters require its
  crates.io version.
- Register the crate in AQC release configuration.
- Existing Package JSON and TSConfig behavior remains unchanged; only their
  calls to the shared scalar reconciler supply the identity it previously
  derived.

## Fixture Families

- strict parsing and byte preservation;
- scalar, list, glob, object, root, and ancestor conflicts;
- missing-value generation and idempotence;
- malformed containers and blocked parents;
- invalid-glob atomic failure;
- overlapping-glob attribution and duplicate-value deduplication;
- RFC 6901 identities and `$` root identity.

The `generic-json-file-engine` Fixture3 suite locks emitted finding identities,
generated bytes, idempotence, invalid-glob atomicity, empty requirements, and
ancestor/descendant object behavior. Rust tests additionally cover merge-only
contracts and parser edge cases.

## Implementation Stops

- Do not add arbitrary recursive JSON values or mixed-array reconciliation.
- Do not add tool-specific settings or file paths.
- Do not duplicate core assertion or JSON CST behavior.
- Do not change Package JSON or TSConfig finding identities.

## Review Result

- Initial review found eight gaps.
- The architecture now requires closure/descendant conflict detection,
  preflight glob compilation, explicit finding keys, constructive object
  initialization, combined glob attribution, exact scope verification, and
  cold-cache-safe gates.
- Follow-up review found duplicate same-surface presence conflicts. Core now
  owns scalar-to-scalar and object-key-to-object-key conflicts; JSON presence
  resolution owns only cross-surface contradictions, and JSON object closure
  ignores child membership owned by that same object requirement.
- Kind-conflict suppression applies only when opposite presence requirements
  carried by the two specific kind occurrences explain every incompatible
  pair; repeated policy provenance cannot conflate separate requirements.
- Forbidden and nonconstructive occurrences are compatible because absence
  satisfies both; they neither create nor prevent a required kind conflict.
- Kind conflicts attribute only occurrences participating in unexplained kind
  pairs; contributors covered by a separate presence conflict are omitted.
- Core `push_rendered_conflict` sorts rendered contributors; generic and
  custom conflicts use the same deterministic construction path.
- Generic JSON merge collection, conflict classification, glob conflicts, and
  resolution live in separate private modules under one public engine facade.
- A compatibility wrapper for arbitrary string-dereferencing key wrappers was
  rejected because supported keys are `str`, `String`, and explicit
  format-specific `FindingKey` implementations, and compatibility aliases are
  forbidden.
- Final confirmation review reported no remaining plan, API, behavior,
  fixture, scope, or verifier gap.

## Verification

- Specular checks every public type, field, enum case, export, dependency
  boundary, forbidden API, and planned file.
- Tests prove scalar/list/glob merge conflicts, format-correct collection
  child identities, and deterministic attribution.
- Tests prove strict syntax rejection, missing-file generation, preservation,
  nested paths, non-object parents, wrong list shapes, non-string members,
  contains/excludes/exact, forbidden globs, selectors, and idempotence.
- Fixture3 proves stable emitted findings and generated bytes for the generic
  behavior contract.
- `cargo test --all-features --locked`, `cargo clippy --all-targets
  --all-features --locked`, `cargo deny check`, and package gates pass for both
  changed workspaces.

## Decisions

- Do not add recursive arbitrary JSON assertions. None of the approved
  consumers requires arbitrary object or mixed-array reconciliation.
- Do not infer object closure from addressed paths. Callers request closure
  explicitly with `object_keys` and core `ItemRequirements<KeyedItem<()>>`.
- Do not generalize Syncpack ordered groups into this engine.
- Do not duplicate list or glob models in the JSON layer.
