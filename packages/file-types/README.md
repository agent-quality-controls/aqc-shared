# file-types

Syntax families. Each subfolder is a future `aqc-*-parser` / `aqc-*-engine`
crate pair, one per concrete file kind (Cargo.toml, clippy.toml, ...).

There is **no format-layer wrapper crate**. Each parser/engine depends on
the underlying grammar crate directly. The list below is which crate to use
for each grammar - if it's already maintained upstream, reuse it instead of
wrapping or re-implementing.

| Folder | Read grammar | Reconcile grammar | Schema source for domain parsers |
|---|---|---|---|
| `toml/` | `toml` + serde | `toml_edit` | Prefer upstream typed crate per file (e.g. `cargo-util-schemas` for `Cargo.toml`); generate when upstream has no schema crate (e.g. `clippy.toml`); hand-roll only when neither is possible. |
| `json/` | `serde_json` | `serde_json` (re-serialize is fine; we own these files) | Hand-roll typed structs (e.g. `G3rsConfig`). Shape spec: [`g3-workspace-config-format`](https://github.com/agent-quality-controls/guardrail3/blob/development/.plans/g3v2-architecture/g3-workspace-config-format.md). |
| `jsonc/` | `jsonc-parser` | `jsonc-parser` | Ecosystem files (`package.json`, `tsconfig.json`, ...); domain parsers per file. |
