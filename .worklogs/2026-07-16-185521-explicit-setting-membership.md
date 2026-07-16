# Explicit Setting Membership

## Summary

Unified explicit required, forbidden, and exact key membership in file-engine core and migrated the affected JSON, TOML, and YAML engines. Added a permanent source architecture checker, repository gates, CI, Specular coverage, and adversarial fixtures that reject inferred membership and hidden requirement surfaces.

## Decisions Made

- Policies own non-neutral membership; engines derive value constraints only for conflict detection and never turn those constraints into extra findings.
- `resolve_key_membership` combines explicit and derived inputs internally for conflict checking, while its resolved output contains only explicit membership.
- TOML and YAML cores own format-level direct-key reconciliation. YAML removal preserves anchors required by retained aliases and invalid merges stop child reconciliation.
- Adapters may use a direct neutral `ItemRequirements::default()` for an independent engine field, but may not construct non-neutral membership or replace policy-supplied membership.
- The `aqc-requirement-architecture` tool inventories reachable requirement roots and rejects closure flags, local core vocabulary copies, opaque root macros, and adapter membership construction or mutation.
- The permanent Specular custom verifier owns checker tests and live repository scans. Shell and CI gates invoke Specular instead of duplicating checker execution.
- Adapter membership fields use a positive rule: direct policy transfer or `ItemRequirements::map`. Unknown local and cross-crate helper calls are rejected.
- Permanent custom-case declarations are consumed by verifier evidence checks. CI installs Specular and cargo-deny and permits clean-cache dependency resolution.
- AQC gates reject downstream Shackles vocabulary instead of enumerating downstream crate names in upstream `deny.toml` files.
- The checker and its independent Cargo fixtures carry repository marker files so the existing commit hook can verify their workspace boundaries.

## Key Files For Context

- `.plans/2026-07-16-144118-explicit-setting-membership.md`
- `packages/aqc-file-engine-core/src/merge/items.rs`
- `packages/file-types/toml/aqc-toml-engine-core/src/table_keys.rs`
- `packages/file-types/yaml/aqc-yaml-engine-core/src/runtime/root_keys.rs`
- `tools/aqc-requirement-architecture/src/analyze.rs`
- `tools/aqc-requirement-architecture/src/expression.rs`
- `scripts/check-workspaces.sh`
- `specs/explicit-setting-membership.spec.json`

## Next Steps

- Pin this AQC commit in Shackles CI and run the Shackles permanent cross-repository contract.
