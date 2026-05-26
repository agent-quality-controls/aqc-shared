# file-types

Syntax families. Each subfolder is a future `aqc-*` crate.

Per family: `aqc-{format}-parser` → `aqc-{domain}-parser` → `aqc-{domain}-engine`.

| Folder | Grammar | Typical domains |
|--------|---------|-----------------|
| `toml/` | `toml_edit` | Cargo, clippy, rustfmt, … (tool configs) |
| `json/` | strict JSON (`serde_json`) | `g3rs.json`, `g3ts.json` — shape in [g3-workspace-config-format](https://github.com/agent-quality-controls/guardrail3/blob/development/.plans/g3v2-architecture/g3-workspace-config-format.md) |
| `jsonc/` | `jsonc-parser` | `package.json`, `tsconfig.json`, … (ecosystem) |
