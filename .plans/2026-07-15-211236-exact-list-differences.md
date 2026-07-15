# Exact List Difference Unification

## Status
`IMPLEMENTATION-READY`

## Goal
- Keep `ListRequirements::{contains, excludes, exact}` as the complete list requirement vocabulary.
- Make exact-list findings identify missing and unexpected members with selectors.
- Keep a selectorless finding only when the list has the correct multiset in the wrong order.
- Reuse one core difference calculation in JSON, TOML, and YAML engines.
- Let a standard waiver suppress one exact-list member without suppressing unrelated members.
- Preserve exact-list merge, desired-byte, and create-missing-list behavior.

This is the AQC prerequisite for the Shakts CSpell plan. CSpell needs exact-empty suppression lists whose intentional exceptions remain individually waivable.

Excluded:
- no new assertion verb or compatibility alias;
- no `allowed`, `closed`, or format-specific list requirement;
- no change to scalar, item, map, or forbidden-glob semantics;
- no file paths, IO, policy meaning, adapter meaning, or Shackles dependency in AQC;
- no publication in the implementation round unless separately requested.

## Evidence
### Current Core
`ListRequirements` already expresses:
```rust
pub struct ListRequirements {
    pub contains: BTreeMap<String, String>,
    pub excludes: BTreeMap<String, String>,
    pub exact: Option<(Vec<String>, String)>,
}
```

Core resolution already:
- merges equal exact lists with provenance;
- conflicts unequal exact lists;
- conflicts exact lists with incompatible contains/excludes;
- exposes `ResolvedExactList` with merged values and attribution.

The requirement model is sufficient. The defect is reconciliation reporting.

### Current Reconciliation Duplication
- `aqc-toml-engine-core` emits one selectorless mismatch for any exact-list difference.
- `aqc-json-file-engine` independently emits the same whole-list mismatch.
- `aqc-pnpm-workspace-yaml-engine` independently emits one collection mismatch.
- all three compute desired exact bytes separately after reporting.

The whole-list finding makes a waiver overbroad: waiving one intentional extra member suppresses every other difference in that list.

### Comparable Member Findings
Contains, excludes, and forbidden-glob requirements already report one member identity at a time. Standard Shackles waivers already consume finding selectors. Exact-list differences should use the same identity boundary where membership, rather than order, is wrong.

## Required Behavior
### D1: Difference Calculation
Add one universal core result and function:
```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExactListDifference {
    missing: BTreeMap<String, usize>,
    unexpected: BTreeMap<String, usize>,
    order_mismatch: bool,
}

pub fn exact_list_difference(
    current: &[String],
    expected: &[String],
) -> ExactListDifference;

impl ExactListDifference {
    pub const fn missing(&self) -> &BTreeMap<String, usize>;
    pub const fn unexpected(&self) -> &BTreeMap<String, usize>;
    pub const fn order_mismatch(&self) -> bool;
    pub fn is_empty(&self) -> bool;
}
```

Rules:
- counts are multiset differences, not set differences;
- `missing[value]` is expected count minus current count when positive;
- `unexpected[value]` is current count minus expected count when positive;
- `order_mismatch` is true only when both count maps are empty and sequences differ;
- equal lists return empty maps and `false`;
- result ordering is lexical through `BTreeMap`;
- every stored count is positive;
- `order_mismatch` cannot coexist with missing or unexpected members;
- empty strings and duplicate values are valid identities and retain correct counts;
- the function performs no rendering and creates no findings.

Examples:
```text
current [b], expected [a]
  missing {a:1}, unexpected {b:1}, order false

current [a,a], expected [a]
  missing {}, unexpected {a:1}, order false

current [b,a], expected [a,b]
  missing {}, unexpected {}, order true

current [], expected []
  missing {}, unexpected {}, order false
```

### D2: Finding Contract
Every list reconciler uses the shared difference result.

When the list field itself is absent:
- emit the existing selectorless missing/exact-list finding;
- do not also emit member differences;
- initialize the complete expected list, including exact empty;
- a field-level waiver remains distinct from a member waiver.

For each distinct missing value:
- finding selector is that value;
- current says absent or includes the lower count;
- expected says present or includes the required count;
- message and attribution come from the resolved exact assertion.

For each distinct unexpected value:
- finding selector is that value;
- current says present or includes the extra count;
- expected says absent or includes the allowed count;
- message and attribution come from the resolved exact assertion.

When only order differs:
- emit one selectorless exact-order finding;
- render complete current and expected lists;
- use exact assertion message and attribution.

Do not emit a second selectorless exact mismatch when member findings already explain the difference.

Compatible exact and member assertions remain separate findings:
- exact plus `contains` may emit two missing-member findings for one identity;
- exact plus `excludes` may emit two unexpected-member findings for one identity;
- each finding keeps its own message and attribution;
- both use the same format-specific key and selector identity;
- one standard key/selector waiver suppresses both findings for that identity without suppressing siblings.

Format engines retain ownership of:
- finding key syntax;
- current/expected rendering;
- severity;
- collection mutation;
- format-specific list shape errors.

### D3: Waiver Consequences
- A selector waiver for one unexpected member suppresses only that member finding.
- A sibling unexpected or missing member remains visible.
- A selector waiver does not suppress an order-only finding.
- A selectorless waiver may suppress the order-only finding under existing waiver semantics.
- Waivers do not change expected bytes or merge semantics.

### D4: Reconciliation Consequences
- Desired bytes remain the resolved exact list after findings are created.
- Missing exact-empty lists remain constructive and initialize as empty arrays/sequences.
- Wrong list value kinds keep the existing shape finding; no member difference is emitted when members cannot be read.
- Contains/excludes findings remain unchanged. Compatible overlap with exact remains separately attributed as defined in D2; contradictory combinations still fail merge.
- Forbidden-glob findings remain member-specific and may coexist with compatible exact findings only where current bytes violate both requirements; each requirement retains its own attribution.

## Package Scope
### Core Runtime/API
Change `aqc-file-engine-core`:
- add `ExactListDifference` beside list requirement/resolution types;
- add `exact_list_difference` in list merge/support code;
- re-export both from the facade;
- add unit tests for equality, missing, unexpected, replacement, duplicates, empty strings, order only, and deterministic ordering.

Version: `0.7.2`. The helper is additive and the changed diagnostics correct overbroad exact-list findings without changing requirement resolution or expected bytes.

### TOML
Change `aqc-toml-engine-core`:
- replace local whole-list comparison reporting with the core difference result;
- preserve `ListFieldKeyStyle` and TOML rendering;
- add presence-aware entry points:
```rust
pub fn reconcile_optional_list_field(
    display_key: String,
    current: Option<Vec<String>>,
    requirements: &ResolvedListRequirements,
    key_style: ListFieldKeyStyle,
    findings: &mut Vec<Finding>,
) -> Option<Vec<String>>;

pub fn reconcile_optional_table_list_field(
    display_key: String,
    current: Option<Vec<String>>,
    requirements: &ResolvedListRequirements,
    findings: &mut Vec<Finding>,
) -> Option<Vec<String>>;
```
- retain `reconcile_list_field` and `reconcile_table_list_field` only as lower-level known-present APIs;
- make optional entry points handle absence once, then delegate member/order reconciliation to the known-present primitive;
- migrate every concrete caller that reads an optional TOML field without erasing `None` to an empty vector;
- add contract tests for member selectors, duplicate counts, order-only mismatch, exact-empty initialization, attribution, and no duplicate whole finding.

Runtime callers change in Cargo TOML, deny TOML, rust-toolchain TOML, and rustfmt TOML. Clippy does not reconcile `ListRequirements` exact values and needs no runtime change. Add regressions wherever an engine transforms list findings, keys, canonical order, or missing-field writes.

### JSON
Change `aqc-json-file-engine`:
- replace `push_list_findings` exact whole-list branch with the core difference result;
- keep RFC 6901 path keys and put member identity in `selector`;
- add exact-empty, sibling-waiver-ready, duplicate, empty-string, and order tests.

Package JSON and TSConfig do not use this generic list reconciliation but move to the coherent core dependency generation.

### YAML
Change `aqc-pnpm-workspace-yaml-engine`:
- replace its exact collection mismatch with the core difference result;
- keep YAML shape/render/write behavior;
- add member, duplicate, order-only, attribution, and desired-output tests.

`aqc-yaml-engine-core` moves to the coherent core dependency generation; no YAML-core runtime behavior changes.

### Dependency Generation
The API addition is backward-compatible and the behavior change fixes overbroad diagnostics without changing resolved requirements or expected bytes. Use one patch generation:
- `aqc-file-engine-core 0.7.2`;
- `aqc-json-file-engine 0.1.1`;
- `aqc-toml-engine-core 0.8.1`;
- `aqc-pnpm-workspace-yaml-engine 0.7.2`;
- Cargo, deny, rust-toolchain, and rustfmt TOML engines at `0.7.2`.

Each changed crate requires `aqc-file-engine-core >=0.7.2, <0.8.0` through normal Cargo `0.7.2` dependency syntax. Existing published consumers requiring `0.7.1` accept `0.7.2`, so Cargo resolves one core generation. Existing TOML engines requiring `aqc-toml-engine-core 0.8.0` accept `0.8.1`; the existing pnpm adapter requiring YAML engine `0.7.1` accepts `0.7.2`; existing Prettier consumers requiring JSON file engine `0.1.0` accept `0.1.1`.

Existing adapters require the affected engine `0.7` lines and accept these patch releases, so no Shackles dependency migration is needed. Do not bump or republish dependency-only JSON, text, Clippy TOML, YAML-core, or Shackles packages. Their declared ranges already consume the fixes without duplicate core versions. Regenerate locks only in changed workspaces and verification consumers that must prove the selected generation.

Publication order when requested:
1. `aqc-file-engine-core 0.7.2`;
2. `aqc-toml-engine-core 0.8.1`;
3. JSON file, pnpm YAML, Cargo TOML, deny TOML, rust-toolchain TOML, and rustfmt TOML patch releases;
4. downstream Shackles adapters, policies, and CLIs.

Local source replacement is verification-only. Committed manifests and registry locks contain no path dependencies.

## Verification
### Core Tests
- every difference example and duplicate count;
- lexical output order independent of input insertion;
- no format-specific contract leaks into the API;
- existing list resolution tests unchanged except imports.

### Engine Tests
For JSON, TOML, and YAML:
- one unexpected value gives one selector finding;
- two unexpected values give two independently suppressible selectors;
- one missing and one unexpected value both remain visible;
- compatible exact-plus-contains and exact-plus-excludes retain two separately attributed findings with the same waiver identity;
- duplicate count mismatch is reported once with count data;
- order-only mismatch is selectorless;
- exact-empty missing field initializes successfully;
- a missing exact list emits one selectorless field finding and no member findings;
- wrong shape emits only shape finding;
- output bytes equal the exact list;
- attribution includes every exact contributor in deterministic order.

### Fixture3
Add AQC probe fixtures for serialized findings and expected bytes in all three formats. Shackles waiver behavior is proved in the downstream CSpell fixture because AQC does not own waiver application.

### Specular
The AQC spec must prove:
- exact changed-file scope;
- core API definitions and exports;
- all direct dependency migrations and crate versions;
- no `allowed`/`closed` vocabulary or aliases;
- all three reconcilers call the core helper;
- every optional TOML list caller preserves absence until the presence-aware core entry point;
- no remaining local exact-list multiset/order implementation;
- required test and fixture families;
- no Shackles dependency or product vocabulary.

## Decisions
Accepted:
- improve exact-list diagnostics rather than add an allowed-item assertion;
- require exact-empty suppression lists downstream;
- keep finding construction in format engines;
- add one presence-aware TOML layer over the existing known-present reconciler;
- preserve separately attributed compatible member assertions.

Rejected:
- `allowed` item requirements: unnecessary new algebra when exact-empty lists express the product state;
- generic finding construction in core: finding keys and rendering remain format-owned;
- JSON-only exact-list behavior: repeats a universal operation and leaves format behavior inconsistent;
- whole-list waiver: permits unrelated list drift;
- compatibility fields, aliases, and dual core versions.

## Implementation Stops
Return to architecture if implementation requires:
- a new assertion verb;
- format or product names in file-engine core;
- finding construction in core;
- different difference semantics by format;
- a committed path dependency;
- an engine that cannot preserve its current exact desired bytes;
- a caller left on core 0.7 in the coordinated downstream graph.

## Review Result
- Review found that exact-empty TOML fields lost absence information and that compatible exact/member assertions can produce separately attributed findings.
- Corrections add presence-aware TOML entry points over the known-present reconciler, migrate every optional caller, preserve same-identity compatible findings, and define patch releases for every runtime caller.
- Confirmation review found no remaining architectural blocker.
