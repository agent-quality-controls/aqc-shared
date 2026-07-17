# Explicit Setting Membership

## Goal

- Remove every engine `exact_settings` and `closed_settings` flag.
- Represent required, forbidden, and exact file-key membership only with `ItemRequirements<KeyedItem<()>>`.
- Keep value requirements separate from key membership.
- Reject value and membership requirements that cannot be satisfied together.
- Reuse one core item-presence comparison across JSON, TOML, YAML, and Cargo reconciliation.
- Add a permanent AST-based architecture gate that discovers future requirement roots and rejects semantic closure flags.

## Evidence

- `ItemRequirements::exact` is an explicit complete present collection. It is constructive and participates in required/forbidden conflict resolution.
- The four live `exact_settings` flags have different behavior:
  - Deny uses fixed optional allowlists for the document root and `[bans.build]` only.
  - Rustfmt derives an optional allowlist from represented scalar/list/glob requirements.
  - Rust toolchain permits every supported `[toolchain]` key whether requested or not.
  - pnpm derives an effective YAML-key allowlist from represented requirements.
- Generic JSON already models object membership explicitly with `ItemRequirements<KeyedItem<()>>`.
- JSON object reconciliation and Cargo lint-table reconciliation duplicate the same required/forbidden/exact presence comparison.

## Decisions

### Exact keeps one meaning

`exact` always names the complete present collection.

```rust
pub struct ItemRequirements<Item> {
    pub required: Vec<ItemAssertion<Item>>,
    pub forbidden: Vec<ItemAssertion<Item>>,
    pub exact: Option<ExactItems<Item>>,
}
```

- `exact` remains constructive and is never used as an optional allowlist.
- Optional-but-allowed membership is modeled by the identity-only `allowed` collection specified in `2026-07-17-051043-allowed-item-membership.md`.
- No closure flag, alias, or compatibility field is added.

### Core compares item presence

Add one format-neutral result and function in `aqc-file-engine-core`:

```rust
pub struct ItemPresenceDifference<'a, Item: FileItemRequirement> {
    pub missing: Vec<(&'a Item::Identity, &'a RequiredItemResolution<Item>)>,
    pub forbidden: Vec<(&'a Item::Identity, &'a ForbiddenItemResolution<Item>)>,
    pub unexpected: Vec<&'a Item::Identity>,
}

pub fn item_presence_difference<'a, Item>(
    current: &'a BTreeSet<Item::Identity>,
    requirements: &'a ResolvedItemRequirements<Item>,
) -> ItemPresenceDifference<'a, Item>;
```

- Missing includes required and exact members once.
- Forbidden includes present explicitly forbidden members.
- Unexpected includes present members outside an exact set.
- The result retains references to resolved requirements so format engines preserve messages and provenance.
- Add `ItemRequirements::map` to transform every required, forbidden, and exact item while preserving messages and structure. Adapters use this instead of reconstructing exact collections.
- A present explicitly forbidden identity is not also reported as unexpected when exact membership excludes it.

### Value requirements constrain key membership

Core exposes `FileKeyRequirement` and `resolve_key_membership`.

- Scalar assertions other than `Absent` require their file key; `Absent` forbids it.
- A list with required members or an exact list requires its file key.
- An item collection with required members or a nonempty exact collection requires its file key. Exact empty item collections remain satisfied by an absent container where format reconciliation already defines that behavior.
- Core resolves explicit membership for output and resolves explicit plus derived constraints together for conflict detection. Callers cannot omit the explicit side of cross-conflict validation.
- Explicit membership alone becomes the resolved reconciliation input, so derived constraints cannot produce duplicate key and value findings.
- Existing item conflict resolution rejects values excluded by exact membership and values whose keys are explicitly forbidden, with policy provenance.

### Format mechanics stay format-specific

- JSON keeps JSON Pointer keys and JSON object mutation, but uses the core presence difference.
- TOML core adds two-phase table-key reconciliation over `ResolvedItemRequirements<KeyedItem<()>>`; concrete engines reject forbidden or unexpected parent keys before child-value reconciliation, then report keys still missing afterward.
- YAML core adds reusable effective-root-key reconciliation; inherited extras remain findings and are not rewritten through anchors.
- YAML root membership runs in two phases: remove and report present forbidden or unexpected keys before child reconciliation, then report missing required keys afterward.
- Child reconciliation does not report a second shape or value finding for a key already rejected by root membership.
- Cargo lint-table reconciliation uses the core presence difference without changing Cargo table semantics.

### Engine requirement surfaces

- Rustfmt replaces `exact_settings` with `setting_keys: ItemRequirements<KeyedItem<()>>` and the resolved equivalent.
- Rust toolchain replaces it with `toolchain_keys: ItemRequirements<KeyedItem<()>>`.
- Rust toolchain rejects exact empty `[toolchain]` membership during requirement merge because `rust-toolchain.toml` requires a nonempty `[toolchain]` table.
- pnpm replaces it with `root_keys: ItemRequirements<KeyedItem<()>>`; comparison uses effective YAML keys.
- Deny replaces one partial flag with `table_keys: BTreeMap<DenyTable, ItemRequirements<KeyedItem<()>>>`.
- `DenyTable` is a closed enum for every modeled Deny table path: root, graph, output, advisories, licenses, licenses private, bans, bans workspace dependencies, bans build, sources, and sources allow-org.
- Deny applies key requirements at every requested table, rather than silently closing only two scopes.
- Constructive explicit membership in a nested Deny table requires that table and every parent table; impossible parent/child membership conflicts during merge.
- Missing exact keys that child value reconciliation cannot construct produce existing unwritable-required-key findings.

## Permanent Architecture Gate

Add an independent Rust checker under `tools/aqc-requirement-architecture` using `syn` and `cargo_metadata`.

- Discover public named-field structs implementing `EngineRequirement` or `AdapterRequirement` in supplied repository roots; reject aliased, tuple, unit, and unresolved roots.
- Inspect every requirement root, all private and public root fields, and every local child struct reachable from a root. Resolved output types outside the input graph are not requirement roots.
- Reject public closure-marker fields such as `exact_settings`, `closed_settings`, or equivalent exact/closed membership fields whose type is not an explicit core collection.
- Reject every adapter construction of non-neutral `ItemRequirements` and every adapter mutation of membership, including assignment, mutable borrowing, helper parameters, arbitrary local names, and mutating method calls. Adapters may pass an identical explicit collection through unchanged, transform item types through `ItemRequirements::map`, or place `ItemRequirements::default()` directly in an independent engine field that has no policy membership input.
- Classify adapter crates by package role as well as discovered traits, so a renamed trait import cannot disable expression checks.
- Inspect local macro bodies and reject membership construction there. Reject module-level macro invocations in requirement crates because unexpanded macros can hide requirement roots from inventory.
- Resolve renamed requirement-trait imports for root inventory.
- Accept only direct `ItemRequirements<KeyedItem<()>>` membership fields or `BTreeMap<Scope, ItemRequirements<KeyedItem<()>>>`; arbitrary wrappers and nested item types do not count.
- Require public fields named `*_keys` to use that canonical membership shape, which catches imported aliases and named wrappers that syntax-only alias resolution cannot inspect.
- Reject adapter destructuring of `required`, `forbidden`, or `exact`.
- Reject imported renames of canonical core membership types; requirement vocabulary uses the established core names without aliases.
- Reject replacement of policy-supplied membership through default construction, membership hidden behind noncanonical public field names, and local macro aliases.
- Restrict mutation checks to membership-shaped receivers so unrelated fields named `required`, `forbidden`, or `exact` remain valid.
- Track membership values through local bindings and `ItemRequirements::map`, independent of variable names.
- Derive membership field names from indexed field types rather than suffix heuristics; unrelated fields such as `cache_keys` remain outside the rule.
- Seed membership tracking from typed function parameters and reject local reimplementations of `ItemRequirements` or `KeyedItem` in requirement crates.
- Track assignments and mutable borrows through dereferences. Reject ambiguous reachable local child type names instead of choosing one unqualified declaration and leaving another uninspected.
- Inspect private and public fields of public requirement roots so private closure markers cannot hide semantics.
- External macros and token words unrelated to membership remain allowed; local macro checks resolve canonical membership aliases instead of treating `required`, `forbidden`, and `exact` words alone as membership. Adapter reconciliation tests provide the output-level transfer proof that source-only macro inspection cannot provide.
- Emit a machine-readable inventory of checked requirement roots and membership fields.
- Resolve requirement traits through canonical cross-crate public re-exports, and require the permanent spec to enumerate every production requirement root so an empty or partial scan fails.
- Include adversarial checker fixtures proving each renamed flag, alias, wrapper, tuple root, nested expression, macro, and inferred or method-mutated collection produces its own expected failure while policy construction, lossless mapping, and a direct neutral engine-field default pass.
- Run the checker's cargo-deny, strict Clippy, tests, and AQC scan from the AQC local gate. Run the checker against both AQC and Shackles from the downstream Shackles local gate; AQC must not depend on or assume a Shackles checkout.
- The checked-in AQC pre-push hook invokes the complete AQC local gate. Pre-commit remains change-scoped. There is no architecture-only CI workflow.

The checker is universal: it knows requirement traits and core collection shapes, not Shackles products, tools, policies, or file formats.

- AQC manifests, deny files, source, and tool code must not name downstream Shackles products. The AQC gate scans these surfaces so reverse vocabulary coupling fails before commit.
- YAML key removal must preserve every alias used by retained document content. A key that owns a referenced anchor remains in expected bytes and receives the membership finding instead of producing broken YAML.
- The Specular architecture contract is permanent. Its custom verifier runs checker tests and repository scans; repository local scripts and checked-in pre-push hooks invoke Specular rather than invoking the checker as a separate architecture gate.
- The AQC local gate runs cargo-deny, strict Clippy, and the permanent Specular architecture contract. Shackles pins the AQC checker revision used by its local cross-repository gate.

## Behavior Proof

- Core tests: missing required, present forbidden, unexpected exact, exact empty, duplicate identities, compatible/incompatible exact sets, required outside exact, forbidden inside exact, and attribution.
- JSON and Cargo regression tests prove behavior is unchanged when explicit key requirements match previous explicit collections.
- Each migrated engine tests missing, forbidden, extra, absent scalar excluded from exact, conflicting exact sets, init output, second reconciliation, and provenance.
- Each migrated engine rejects a constructive value requirement excluded by exact membership before reconciliation.
- Rustfmt proves `Absent` settings are not exact-present and rejects `allow_nightly_settings` plus exact policy mode downstream.
- Toolchain proves `path` is not silently authorized and empty target input does not require `targets`.
- Toolchain proves exact empty membership fails merge instead of creating an invalid empty table.
- pnpm proves effective YAML merge-key handling, direct extra removal, inherited extra preservation/finding, absence-only/glob-only fields excluded from exact membership, and one root-membership finding for each rejected direct or inherited child key.
- Invalid YAML merge sources stop child reconciliation after the root error so one malformed merge does not create unrelated child findings.
- Removing an unexpected YAML key that defines an anchor used by a retained key leaves the bytes valid and produces no cascading child finding.
- Deny proves every modeled table scope, not only root and `[bans.build]`.
- TOML engines prove one rejected parent key produces one parent finding and one missing required key produces one child-value finding when child reconciliation can construct it.

## Files

- `packages/aqc-file-engine-core/src/merge/{model,item_model,items,forbidden_globs}.rs`, exports, and tests.
- `packages/file-types/json/aqc-json-file-engine/src/runtime/reconcile/document.rs` and tests.
- `packages/file-types/toml/aqc-toml-engine-core` table-key API and tests.
- Cargo lint-table reconciliation and tests.
- Deny requirement, merge, reconcile, exports, and tests.
- Rustfmt requirement, merge, reconcile, exports, and tests.
- Rust toolchain requirement, merge, reconcile, exports, and tests.
- `packages/file-types/yaml/aqc-yaml-engine-core` root-key API and tests.
- pnpm requirement, merge, reconcile, exports, and tests.
- `tools/aqc-requirement-architecture` package and checker fixtures.
- Repository local requirement-architecture gates and checked-in pre-push hooks in AQC and Shackles.
- AQC workspace/check scripts, Specular files, Fixture3 files, plan, and worklog.

## Verification

- Specular lint and pre-implementation failure, then passing verification.
- Every affected workspace: format, test, strict Clippy, cargo-deny, package dry-run, and MSRV gates.
- AQC Fixture3 doctor and all suites.
- Permanent Specular architecture contracts and local live scans against AQC and Shackles source.
- Adversarial review against this plan, spec, public surfaces, behavior, and checker bypasses.
