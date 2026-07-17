# Explicit Setting Membership Coverage

Sources:

- `.plans/2026-07-16-144118-explicit-setting-membership.md`
- `.plans/2026-07-16-205526-fast-local-gates.md`
- `.plans/2026-07-17-051043-allowed-item-membership.md`

## Explicit Setting Membership

- `not-applicable`: document title; requirements are mapped by the headings below.

## Allowed Item Membership

- `content`: public allowed input/resolved types and identity-specific rejection attribution.
- `custom:core-item-presence-semantics`: intersection, constructive conflicts, mapping, and attribution.

## Architecture

- `content`: canonical `allowed` field and resolved membership API.
- `dependencies`: keeps allowed membership format- and product-independent.

## Merge Rules

- `custom:core-item-presence-semantics`: intersections, required/exact conflicts, forbidden compatibility, and failed value composition.

## Reconciliation

- `custom:format-membership-reconciliation`, `custom:migrated-engine-behavior`: optional absence, unexpected removal, one classification, and per-item attribution.

## Consumers

- `tree`, `content`, `custom:format-membership-reconciliation`, `custom:migrated-engine-behavior`: every affected engine path and literal uses the canonical shape.

## Architecture Gate

- `custom:architecture-checker-semantics`: policy construction and adapter transfer/map acceptance, plus adapter-authored membership rejection.

## Goal

- `content`: core API, engine membership fields, universal closure-marker prohibition, and shared reconciliation use.
- `tree`, `dependencies`, `custom:architecture-checker-semantics`: permanent checker package, adversarial behavior, and live repository scan.

## Evidence

- `not-applicable`: records the pre-change observations motivating the plan; it does not define additional final-state requirements.

## Decisions

- `not-applicable`: container heading; each decision subsection is mapped separately.

## Exact keeps one meaning

- `content`: forbids `exact_settings` and `closed_settings` throughout engine source.
- `custom:core-item-presence-semantics`: proves constructive exact membership and required/forbidden conflict behavior.
- `allowed` permits optional identities, intersects across policies, and reports only contributors that exclude each rejected identity.

## Core compares item presence

- `content`: requires `ItemPresenceDifference`, its three result collections, `item_presence_difference`, `ItemRequirements::map`, and facade exports.
- `dependencies`: keeps core independent of format and product crates.
- `custom:core-item-presence-semantics`: executes the complete core behavior matrix and lossless-map proof.

## Value requirements constrain key membership

- `content`: requires `FileKeyRequirement` and `resolve_key_membership` in core and its facade.
- `custom:core-item-presence-semantics`: proves derived constraints detect conflicts without becoming reconciliation membership and suppress duplicate explicit-forbidden/exact findings.
- `custom:migrated-engine-behavior`: proves each migrated engine rejects constructive values excluded by exact membership before reconciliation.

## Format mechanics stay format-specific

- `content`: requires JSON, Cargo, TOML, and YAML integration with the core presence comparison.
- `dependencies`: requires affected format crates to consume `aqc-file-engine-core` without product dependencies.
- `custom:format-membership-reconciliation`: executes format-specific regression, effective-key, mutation, unwritable-key, and parent-removal-before-child finding proofs.

## Engine requirement surfaces

- `content`: requires explicit raw and resolved membership fields for Rustfmt, toolchain, pnpm, and Deny.
- `enumerations`: fixes `DenyTable` to every modeled table path.
- `custom:migrated-engine-behavior`: executes the common engine matrix, rejects impossible exact-empty toolchain requirements before reconciliation, and proves pnpm reports one root-membership finding for a rejected child.

## Permanent Architecture Gate

- `tree`, `dependencies`, `content`: requires the independent checker, `syn`, `cargo_metadata`, universal vocabulary, inventory output surface, local workspace-gate invocation, and checked-in pre-push hook execution.
- `custom:architecture-checker-semantics`: executes exact named adversarial checker tests for local and cross-crate helper production, direct and multi-hop canonical requirement-trait re-exports, helper-parameter laundering, policy-membership discard, same-name destructuring, typed membership locals, canonical core origin through public re-exports, nested and terminal-name counterfeits, unrelated `*_keys` fields, direct transfer, and `ItemRequirements::map`, plus the established renamed-trait, wrapper, macro, mutation, borrowing, and inventory cases. It compares the live production inventory with the complete expected requirement-root set.

## Behavior Proof

- `custom:core-item-presence-semantics`: core cases and attribution.
- `custom:format-membership-reconciliation`: JSON, Cargo, TOML, and YAML cases.
- `custom:migrated-engine-behavior`: Deny, Rustfmt, toolchain, and pnpm cases.
- Fixture3 remains downstream behavior evidence; the AQC plan does not identify deterministic new fixture paths or outputs for a builtin tree/content assertion.

## Files

- `tree`: requires every exact implementation, test, checker, gate, Fixture3 manifest, and Specular path stated or agreed by the three extraction candidates.
- `content`, `dependencies`, `enumerations`, and all custom checks cover the less-specific file groups in the plan.

## Verification

- `specular lint` and the required pre-implementation `specular verify` failure are execution steps reported when this contract is created, not self-referential contract items.
- The four custom checks execute relevant Cargo test suites before returning pass evidence.
- Format, package dry-run, MSRV, Fixture3, and adversarial review remain mandatory plan acceptance gates outside this contract. The permanent Specular verifier owns the AQC self-scan; the Shackles local contract owns the cross-repository scan.

## Fast Local Gates

- `tree`, `content`: stable repository-owned Cargo targets, isolated Cargo configuration, run-scoped logs, staged-index pre-commit snapshots, immutable detached pre-push snapshots, and local hook wiring.
- `custom:architecture-checker-semantics`: manifest-identity origin tracking, custom Cargo targets and nested `#[path]` modules, helper-return and direct, destructured, or referenced closure-alias flows, adapter `self`, imported and qualified local roots, imported semantic aliases, generic and same-terminal local shadows, chained module-scoped aliases, rejected glob or block-local imports, and macro-only requirement roots.
- `custom:format-membership-reconciliation`, `custom:migrated-engine-behavior`: locked tests and strict Clippy for every discovered workspace.
- `.github/workflows/requirement-architecture.yml` is forbidden by the tree requirement; architecture verification is local.
- Atomic Cargo patch generation and clone-local fixture Cargo state are enforced by the gate scripts.
- Acceptance is completed by two Specular runs, all Fixture3 suites, the full local gate, and converged adversarial review.
