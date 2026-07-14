# File Engine Cores

- `aqc-file-engine-core` owns format-neutral assertions, requirement composition, provenance, conflicts, findings, and erased engine dispatch.
- Format cores own reusable parsing and edit mechanics for one syntax. They do not own tool settings or product policy.
- Concrete engines own one file's structure, syntax options, unresolved and resolved requirement roots, reconciliation, and deterministic initialization bytes.
- Engines are pure transforms over supplied bytes and requirements. Paths, filesystem access, commands, and product rules stay outside AQC engines.

## JSON Families

- `aqc-json-engine-core` owns lossless JSON-family object parsing and targeted edits. Callers provide `JsonParseOptions`; the core does not select strict JSON or a tool dialect.
- `aqc-package-json-engine` owns `package.json` fields.
- `aqc-tsconfig-json-engine` owns `tsconfig.json`, including TypeScript's JSONC syntax switches and compiler-option keys.

## Attribution

- `ResolvedRequirement::attribution()` is the universal projection from composed assertions to provenance.
- `resolved_map_attribution()` aggregates that projection across addressed map requirements.
- Format and concrete engines construct their own findings because they own keys, selectors, current values, and file-specific messages.
- Engines must not duplicate attribution projection or add alternate mismatch constructors.
