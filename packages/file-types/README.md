# file-types

Syntax families. Universal format mechanics live in an engine-core crate;
concrete file engines add only the requirements and behavior of one file kind.

Concrete engines depend on their format engine core instead of importing or
reimplementing parser mechanics. Format cores keep parser dependencies private.

| Folder | Read grammar | Reconcile grammar | Schema source for domain parsers |
|---|---|---|---|
| `toml/` | `toml` + serde | `toml_edit` | Prefer upstream typed crate per file (e.g. `cargo-util-schemas` for `Cargo.toml`); generate when upstream has no schema crate (e.g. `clippy.toml`); hand-roll only when neither is possible. |
| `json/` and `jsonc/` | `aqc-json-engine-core` | `aqc-json-engine-core` lossless CST | Concrete engines select strict JSON or the exact JSONC syntax accepted by their file format. `package.json` uses strict JSON; `tsconfig.json` enables its supported JSONC extensions. |
