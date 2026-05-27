# Build the engines and shared core

## Summary

Implemented steps 1-3 of the vertical-slice plan:

- `packages/aqc-file-engine-core/` - shared framework types and the
  `FileEngine` trait.
- `packages/file-types/toml/aqc-cargo-toml-engine/` - reconcile engine
  for `[lints.<tool>]` tables in `Cargo.toml`.
- `packages/file-types/toml/aqc-clippy-toml-engine/` - reconcile engine
  for `msrv`, thresholds, and `disallowed-methods` in `clippy.toml`.

All three crates compile clean. `cargo clippy --workspace -- -D warnings`
passes. `cargo fmt --check` clean. The manifest verifier suite exits 0
on every layer (55 checks total).

## Workspace

Added top-level `Cargo.toml` declaring a workspace with the three
engine crates as members, plus `[workspace.lints]` enforcing
`warnings = deny`, `clippy::all = deny`, `clippy::pedantic = deny` with
a few common allows for ergonomics (`module_name_repetitions`,
`missing_errors_doc`, `missing_panics_doc`).

Added `rust-toolchain.toml` pinning to the stable channel with
`clippy` + `rustfmt` components.

## Manifest verifier output

Ran from `aqc-shared/`:

```
$ ./scripts/verify-all.sh
=== layer 1 (tree) ===  PASS  (6 checks)
=== layer 2 (public_api) ===  PASS  (18 checks)
=== layer 3 (closed_sets) ===  PASS  (6 checks)
=== layer 4 (dependency rules) ===  PASS  (14 checks)
=== layer 5 (structural) ===  PASS  (11 checks)
=== ALL LAYERS PASS ===
exit 0
```

## Files added

- `Cargo.toml` (workspace)
- `rust-toolchain.toml`
- `packages/aqc-file-engine-core/Cargo.toml`
- `packages/aqc-file-engine-core/src/lib.rs`
- `packages/file-types/toml/aqc-cargo-toml-engine/Cargo.toml`
- `packages/file-types/toml/aqc-cargo-toml-engine/src/lib.rs`
- `packages/file-types/toml/aqc-clippy-toml-engine/Cargo.toml`
- `packages/file-types/toml/aqc-clippy-toml-engine/src/lib.rs`

## Exported types

### `aqc-file-engine-core`

- `pub type PolicyId = String`
- `pub struct Provenance { policy: PolicyId }`
- `pub struct MergedAssertion<A> { contributions: Vec<(Provenance, A)> }`
- `pub enum Severity { Error, Warning, Info }`
- `pub struct EngineOutput { expected_bytes: Vec<u8>, findings: Vec<Finding> }`
- `pub enum Finding { Mismatch { ... }, UnwritableRequiredKey { ... }, SchemaError { ... } }`
- `pub enum EngineError { Parse(String), Other(String) }` - impls `Display`, `Error`
- `pub struct MergeConflict { target: String, contributors: Vec<Provenance>, detail: String }`
- `pub trait FileEngine<Req> { fn reconcile(current_bytes: Option<&[u8]>, requirement: &Req) -> Result<EngineOutput, EngineError>; }`

### `aqc-cargo-toml-engine`

- `pub struct CargoTomlRequirement { lints: BTreeMap<String, MergedAssertion<LintLevelsAssertion>> }`
- `pub enum LintLevelsAssertion { Contains(BTreeMap<String, String>), Excludes(BTreeSet<String>), IsExactly(BTreeMap<String, String>) }`
- `pub struct CargoTomlEngine`
- `impl FileEngine<CargoTomlRequirement> for CargoTomlEngine`

### `aqc-clippy-toml-engine`

- `pub struct ClippyTomlRequirement { msrv, method_bans, thresholds: Option<MergedAssertion<...>> }`
- `pub enum MsrvAssertion { Equals(String), AtLeast(String), OneOf(BTreeSet<String>), Present, Absent }`
- `pub enum MethodBansAssertion { Contains(Vec<MethodBanEntry>), Excludes(BTreeSet<String>), IsExactly(Vec<MethodBanEntry>) }`
- `pub struct MethodBanEntry { path: String, reason: String }`
- `pub enum ThresholdsAssertion { Equals/AtMost/AtLeast(BTreeMap<String, u64>), Present(BTreeSet<String>), Absent(BTreeSet<String>) }`
- `pub struct ClippyTomlEngine`
- `impl FileEngine<ClippyTomlRequirement> for ClippyTomlEngine`

## Engine behavior implemented (not just stubs)

Both engines implement real reconcile logic via `toml_edit`:

- **Cargo engine** walks per-tool merged contributions, applies
  `Contains`, `Excludes`, and `IsExactly` semantics against the
  `[lints.<tool>]` table, emits `Finding::Mismatch` per disagreement,
  attribution includes the provenances of every contribution that
  mentioned the affected lint.
- **Clippy engine** handles `msrv` (with `AtLeast` doing dotted-version
  comparison), thresholds (`Equals`/`AtMost`/`AtLeast`/`Present`/`Absent`),
  and `disallowed-methods` (per-entry by `path`, recognizing both inline
  and string entry shapes).

`IsExactly` semantics include the extras-removal pass that drops
on-disk entries not in the union of `IsExactly` contributions.

## Next steps

Steps 4-7 of the vertical-slice plan (clippy linter adapter, clippy
policy, broker, CLI wiring) live in guardrail3. None are blocked on
aqc-shared work after this commit.

When that work begins, the manifest at
`guardrail3/.plans/g3v2-architecture/2026-05-26-191126-clippy-vertical-slice.md.manifest.toml`
will be extended with rows for the new packages, and matching verifier
checks will be added.
