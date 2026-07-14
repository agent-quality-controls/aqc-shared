# JSON Engine Core Consolidation Coverage

Plan SHA256: ccddddfc0bebe67fdf8e05393fe7683c9a370e550ebc942f7c2b55b1146e1f89

- Goal: tree, dependencies, exports, and `active-inventory-clean`.
- Approach: the four approach subsections below map each implementation area.
- Unified JSON core: core content, dependencies, exports, `exact-unified-api`, and `runtime-contracts`.
- Concrete engines: concrete content and dependency blocks, exports, and `runtime-contracts`.
- Remove the duplicate core: forbidden tree and `active-inventory-clean`.
- Verification: `runtime-contracts` runs the exact TypeScript 7.0.2 syntax probe and Rust contracts; split `required-gates-*` checks run format, Clippy, cargo-deny, package, and boundary gates; AQC and Shackles fixtures and migrated specs run as independent mechanical gates because their integration suites exceed Specular's verifier timeout.
- Key Decisions: core API checks, parser dependency boundaries, syntax isolation tests, and malformed-parent contract tests.
- Files To Modify: required and forbidden tree entries cover planned AQC artifacts; `active-inventory-clean` scans active tracked and untracked files in both repositories.
- AQC: built-ins check manifests, dependencies, source content, docs, and trees; parsed manifests enforce publication and dependency sources; migrated specs and release checks enforce downstream inventory.
- Shackles: migrated PNPM/TSC specs check manifests, locks, deny files, docs, fixtures, and downstream artifacts; boundary scripts enforce layering.
- Required End State: every custom check in the consolidation spec jointly enforces the stated end state.
