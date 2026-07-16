# Explicit Setting Membership Coverage

Source: `.plans/2026-07-16-144118-explicit-setting-membership.md`

## Explicit Setting Membership

- `not-applicable`: document title; requirements are mapped by the headings below.

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
- Optional allowlists remain outside the contract because the plan explicitly declines to add that primitive.

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

- `tree`, `dependencies`, `content`: requires the independent checker, `syn`, `cargo_metadata`, universal vocabulary, inventory output surface, workspace-gate invocation, and pull-request/push CI execution.
- `custom:architecture-checker-semantics`: executes adversarial checker tests for renamed traits, key-field wrappers and aliases, private closure fields, local core-type copies, module-level macros, tuple roots, direct/default construction, renamed local bindings, helper parameters, method mutation, mutable borrowing, and destructuring. It also proves canonical imported aliases, unrelated similarly named fields, and unrelated external macros are accepted.

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
- Format, package dry-run, MSRV, Fixture3, and adversarial review remain mandatory plan acceptance gates outside this contract. The permanent Specular verifier owns the AQC self-scan; the Shackles contract owns the cross-repository scan.
