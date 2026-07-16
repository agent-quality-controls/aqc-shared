# Exact List Differences Coverage

- Goal: tree, content, dependencies, exports, behavior fixtures, and workspace gates.
- Universal contract: core types, exact difference behavior, desired-list application, and core tests.
- JSON reconciliation: member selectors, order-only findings, application behavior, and JSON tests.
- TOML reconciliation: shared optional and required APIs, member selectors, order-only findings, and TOML tests.
- YAML reconciliation: member selectors, order-only findings, application behavior, and YAML tests.
- Existing callers: Cargo, Deny, Rust toolchain, and Rustfmt use shared TOML reconciliation.
- Boundaries: dependency checks, forbidden format/product vocabulary in core, and changed-path scope.
- User behavior: Fixture3 covers JSON, TOML, and YAML membership, duplicates, order, and absent exact-empty lists.
- Verification: Specular custom gates run formatting, tests, Clippy, cargo-deny, packaging, dependency generation, and scope checks.

No plan section is uncovered.
