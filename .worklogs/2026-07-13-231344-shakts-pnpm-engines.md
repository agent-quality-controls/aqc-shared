# Summary

Added reusable JSON and YAML format cores, Package JSON and pnpm workspace YAML file engines, and the core 0.7 selector contract. Renamed the concrete text engine without a compatibility alias and moved every affected engine to one core generation.

# Decisions Made

- Reused file-engine-core merge, assertion, provenance, conflict, and finding types instead of adding format-specific copies.
- Used yaml-edit for concrete syntax, anchors, aliases, and writes, with one AQC-owned effective mapping resolver because yaml-edit's merged mapping view does not preserve the required YAML merge semantics.
- Rejects cyclic aliases, undefined aliases, duplicate nested keys, invalid merge sources, unknown tags, and wrong field shapes without recursion overflow.
- Added the standard G3RS adoption marker and dependency allowlist to every new independent workspace and the fixture probe.
- Waived the import-count rule for the existing merge facade because its imports are public re-exports required by the facade boundary.
- Kept engines pathless and filesystem-free; runtime diagnostics use format-neutral document labels.
- Made generated output deterministic, parseable, and stable on a second reconciliation.
- Added AQC Fixture3 public-contract coverage and Specular gates for exact facades, resolved APIs, dependencies, purity, existing-engine behavior, and release inventory.

# Key Files For Context

- `specs/shakts-pnpm-aqc.spec.json`
- `packages/aqc-file-engine-core/src/finding.rs`
- `packages/file-types/json/aqc-json-engine-core/src/lib.rs`
- `packages/file-types/json/aqc-package-json-engine/src/lib.rs`
- `packages/file-types/yaml/aqc-yaml-engine-core/src/lib.rs`
- `packages/file-types/yaml/aqc-pnpm-workspace-yaml-engine/src/lib.rs`
- `fixtures/probes/shakts-pnpm-aqc/src/main.rs`

# Next Steps

- Publish the coordinated AQC 0.7 generation only when a release round is requested.
