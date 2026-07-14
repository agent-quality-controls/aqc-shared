# Goal

Replace the duplicate JSON and JSONC format cores with one lossless `aqc-json-engine-core`. Strict JSON and TypeScript-compatible JSONC are parser configurations of the same byte-preserving object model. Delete `aqc-jsonc-engine-core` without aliases or compatibility shims.

# Approach

## Unified JSON core

- Move the lossless CST object, parser, scalar reconciliation, duplicate-key rejection, BOM preservation, and extended-number preservation from `aqc-jsonc-engine-core` into `aqc-json-engine-core`.
- Keep the established public names `JsonObject`, `JsonParseOptions`, `parse_object_or_report`, and `reconcile_scalar_assertion`.
- Delete the serde `Value` object model and whole-document pretty renderer. `JsonObject::render` becomes the sole renderer.
- Make selector input part of the one scalar reconciliation API because it is the stronger existing contract.
- Require each concrete engine to select `NonObjectParentAction::Preserve` or `Replace` when writing a nested scalar. Package JSON replaces malformed parents for its nested scalar fields. Its keyed maps preserve a malformed map parent and report one parent-shape finding, matching their established non-destructive contract. TSConfig preserves and reports malformed `compilerOptions`.
- Keep parser dependencies private. `serde_json` validates strict JSON and preserves its diagnostics; the lossless CST remains the only object, mutation, and rendering model. Concrete engines select syntax; callers cannot access parser implementation types.
- Reject duplicate object names recursively for every syntax configuration.
- Preserve untouched bytes, comments, whitespace, trailing commas, BOM, supported numeric spelling, TypeScript-supported JavaScript string escapes, and TypeScript-supported vertical-tab/form-feed whitespace.
- Keep parsing, extended-syntax normalization, and CST metadata binding in separate private runtime modules so each implementation unit remains focused.
- Create absent documents deterministically from `{}` plus a trailing newline.
- Raise the unified core and Package JSON engine MSRV from Rust 1.85 to Rust 1.88. The retained lossless CST parser uses language features stabilized in Rust 1.88; preserving 1.85 would require keeping the duplicate serde object model.

## Concrete engines

- Change `aqc-package-json-engine` to call the unified core with strict JSON options: comments, loose names, trailing commas, missing commas, single quotes, hexadecimal numbers, unary plus, extended numbers, and BOM are forbidden.
- Preserve Package JSON finding keys, selectors, messages, and duplicate-key rejection.
- Change `aqc-tsconfig-json-engine` to depend on and import `aqc-json-engine-core`; retain its TypeScript syntax options, including JavaScript string escapes, escaped line continuations, and vertical-tab/form-feed whitespace.
- Keep both concrete requirement roots unchanged.
- Raise Package JSON consumers that still declare Rust 1.85 to Rust 1.88 so their manifests state the dependency chain's true MSRV.

## Remove the duplicate core

- Delete the complete `packages/file-types/jsonc/aqc-jsonc-engine-core` workspace.
- Remove every Cargo, cargo-deny, release, fixture, architecture-document, Specular, and boundary-gate reference to `aqc-jsonc-engine-core` and `aqc_jsonc_engine_core` in AQC and Shackles.
- Do not add an alias, re-export, deprecated package, compatibility feature, or replacement type with the old JSONC names.

## Verification

- Extend JSON-core contract coverage with strict JSON rejection, independent syntax switches, JSONC preservation, recursive duplicate rejection, extended numbers and strings, BOM, replacement, insertion, and deterministic absent-document output.
- Keep Package JSON and TSConfig engine contract suites passing.
- Keep AQC and Shackles TSC Fixture3 suites passing.
- Add a Specular consolidation spec that requires the unified tree and dependencies and forbids the deleted package and vocabulary repository-wide.
- Run format, Clippy, tests, cargo-deny, package assembly, dependency-boundary gates, Specular, and Fixture3.
- Adversarially review the plan, spec, implementation, and fixtures until no gap remains.

# Key Decisions

- Keep the crate name `aqc-json-engine-core`; JSONC is a configured JSON-family syntax, not a second reconciliation architecture.
- Keep a lossless CST for strict JSON too. Reformatting an entire existing JSON file is unnecessary and destructive.
- Keep concrete syntax selection in concrete engines. The shared core knows syntax capabilities but not Package JSON, TSConfig, TypeScript, pnpm, policies, adapters, paths, or filesystems.
- Keep malformed-parent handling explicit because the concrete file contracts differ and neither action is universally correct.
- Retain the current lossless parser temporarily because it is private and actively released; parser replacement is independent of the public core consolidation. Keep `serde_json` only as the maintained strict-JSON validator, not as a second object model.
- Reject replacing the unified core with JSON5 semantics because TypeScript accepts a different syntax set.
- Reject retaining Rust 1.85 through a strict-JSON feature backed by serde. That would preserve two parser, object, mutation, and rendering implementations under one crate name instead of consolidating them.

# Files To Modify

## AQC

- `packages/file-types/json/aqc-json-engine-core/**`
- `packages/file-types/json/aqc-package-json-engine/**`
- `packages/file-types/jsonc/aqc-tsconfig-json-engine/**`
- delete `packages/file-types/jsonc/aqc-jsonc-engine-core/**`
- `fixtures/probes/shakts-tsc-aqc/**`
- `fixtures/scripts/fixture3-shakts-tsc-aqc.py`
- `fixtures/approved/shakts-tsc-aqc/**`
- `docs/architecture/file-engine-cores.md`
- `packages/file-types/README.md`
- `.github/workflows/release.yml`
- `release-plz.toml`
- every affected `deny.toml` and `Cargo.lock`
- `specs/json-engine-core-consolidation.spec.json`
- `specs/verifiers/verify-json-engine-core-consolidation.py`
- `specs/json-engine-core-consolidation.spec.coverage.md`

## Shackles

- `AGENTS.md`
- TSC fixture probe locks and approvals when behavior changes
- every affected `deny.toml` and `Cargo.lock`
- dependency-boundary scripts
- active TSC specs and coverage maps that inventory the removed crate

# Required End State

- `aqc-json-engine-core` is the only JSON-family format core.
- Package JSON and TSConfig engines both depend on it.
- No active source, manifest, lock, deny, release, fixture, architecture, or governing-plan inventory in either repository contains the retired package or Rust API vocabulary. The consolidation plan, spec, verifier, coverage map, and worklogs may name the removed vocabulary as historical evidence and enforcement input.
- Both repositories are clean after normal-hook commits.
