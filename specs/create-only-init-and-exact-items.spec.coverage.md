# Create-Only Init And Exact Item Requirements: AQC Coverage

This spec covers the AQC repository portion of the Shackles plan. Create-only runner behavior, adapter and policy lowering, CLI behavior, Fixture3, installed Shakrs, and repository adoption belong to the separate Shackles spec.

## Goal

- Universal `required`, `forbidden`, and `exact` vocabulary: core content blocks and exact-item semantic verifier.
- Validation detail needed by downstream agents: concrete-engine reconciliation blocks and semantic verifier.
- Create-only init: Shackles spec, because AQC engines are byte transforms and do not own files.

## Decisions

### Init owns creation only

- Not an AQC responsibility. The Shackles runner spec owns every filesystem and reporting rule.

### Remove init commands

- Not an AQC responsibility. The Shackles core, runner, and adapter spec owns command-contract removal.

### Exact item vocabulary

- Core model and export content blocks require `exact` and `ResolvedExactItems`.
- Production-wide forbidden-content blocks remove collection `closed` vocabulary.
- `exact-item-semantics` proves identity-set equality, complete composed values, duplicate composition, required/exact composition, forbidden/exact conflicts, multiple exact assertions, messages, provenance, and deterministic attribution.

### Cargo package lint table presence

- Cargo model and implementation content blocks require independent `package_lint_tables` using `ItemRequirements<KeyedItem<()>>`.
- `engine-reconciliation-semantics` proves table identity discovery, inline implication, required/forbidden/exact reconciliation, exact-empty diagnostics, and `[lints.<tool>]` finding keys.
- Policy branch behavior belongs to the Shackles spec.

## AQC Changes

### `aqc-file-engine-core`

- Tree and content blocks require the public and resolved types, exports, merge implementation, tests, and forbidden legacy terms.
- `exact-item-semantics` and `workspace-gates-core` prove behavior and release readiness.

### `aqc-toml-engine-core`

- Tree and content blocks require generic array and array-of-table exact reconciliation.
- `engine-reconciliation-semantics` and `workspace-gates-format-cores` prove behavior and release readiness.

### `aqc-text-engine-core`

- Tree and content blocks require unsupported-exact handling and remove unsupported-closed handling.
- `engine-reconciliation-semantics` and `workspace-gates-format-cores` prove behavior and release readiness.

### Cargo TOML engine

- Tree and content blocks require lint-table presence requirements and exact terminology across dependency, feature, lint, and reconciliation code.
- `engine-reconciliation-semantics` and `workspace-gates-cargo-clippy` prove behavior and release readiness.

### Other AQC engines

- Content blocks cover Clippy item collections and require `exact_settings` in Deny, Rustfmt, and rust-toolchain.
- `engine-reconciliation-semantics`, engine workspace gates, and the production-wide forbidden-content block cover all consumers.

## Shackles Changes

- All Shackles subsections are assigned to the separate Shackles spec.

## Fixture Requirements

- Runner create-only fixtures and Cargo policy conflicts belong to the Shackles spec.
- Exact item and AQC engine regression cases are enforced by `exact-item-semantics` and `engine-reconciliation-semantics` using AQC tests.
- Fixture3 is a Shackles repository gate; AQC behavior uses crate tests because this repository has no Fixture3 manifest.

## Specular Specifications

- Tree requires this AQC spec, coverage map, and verifier.
- Built-ins own tree, content, dependency, version, public-surface, and forbidden-vocabulary checks.
- Custom checks are limited to semantic behavior, workspace gates, release dependency versions, scope, and coverage parity.

### AQC spec

- Every listed AQC requirement is represented by a built-in block or a named custom check.
- `change-scope` limits modifications to affected workspaces and proof artifacts.
- Workspace custom checks run tests, strict Clippy, cargo-deny, package dry-runs, and Rust 1.85 checks.

### Shackles spec

- Not executed from this repository. The separate Shackles spec covers its own files and gates.

## Migration And Release Order

- Manifest content blocks require the approved major/minor versions.
- Dependency built-ins and `release-dependency-versions` require released core use without path dependencies.
- Package dry-runs are part of every workspace gate.
- Interrupted-init cleanup and downstream publishing belong to the Shackles spec and repository-adoption process.

## Resume Repository Adoption

- Not an AQC engine implementation concern. The Shackles spec owns init preservation and adoption commands.
- AQC final validation is an adoption gate after both repositories are released, not a condition for the preimplementation AQC engine spec.

## Completion Gates

- No resolved or unresolved collection `closed` vocabulary: built-in content blocks.
- Exact Cargo lint-table requirements: Cargo content blocks and engine semantic verifier.
- Affected AQC tests, strict Clippy, cargo-deny, package dry-run, and Rust 1.85: workspace custom checks.
- AQC release order and registry dependency propagation: version/dependency built-ins and release custom check.
- Runner init, policy lowering, Fixture3, installed CLI, and final repository validation: separate Shackles spec.
