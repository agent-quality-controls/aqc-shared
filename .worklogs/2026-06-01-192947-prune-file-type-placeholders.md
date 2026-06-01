# Prune pre-generated file-type placeholders

## Summary

Removed every `.gitkeep`-only placeholder under `packages/file-types/`.
These were scaffolded up front to sketch the eventual parser/engine
matrix, but the matrix will be implemented on demand, modeled on the two
engines that already exist (`aqc-cargo-toml-engine`, `aqc-clippy-toml-engine`).
Empty slots add tree noise and imply crates that do not exist, so they
are gone until something real lands in each one.

## What changed

- Deleted the entire `file-types/json/` and `file-types/jsonc/` trees
  (strict-JSON and JSONC placeholders, all `.gitkeep`-only).
- Deleted every `.gitkeep`-only `file-types/toml/` slot (cargo-config,
  cargo-toml-parser, cliff, deny, g3rs-toml, g3ts-toml, mutants, nextest,
  release-plz, rust-toolchain, rustfmt - parser and engine each).
- Deleted two vestigial `.gitkeep` files sitting inside the real
  `aqc-cargo-toml-engine/` and `aqc-clippy-toml-engine/` dirs.
- 37 files total.

## What was deliberately kept

- `aqc-file-engine-core`, `aqc-cargo-toml-engine`, `aqc-clippy-toml-engine`
  (real code).
- `file-types/README.md` (documents the toml/json/jsonc grammar choices).
- `aqc-filetree`, `aqc-fs-utils`, `aqc-git-helpers` (`.gitkeep`-only util
  placeholders outside the parser scope; the runner may want them).

## Decisions made

- The `g3rs.json` parser does NOT live here. It is guardrail3-specific
  (only guardrail3 reads/writes `g3rs.json`), so it belongs in the
  guardrail3 `g3-` layer, not the cross-product `aqc-` shared layer. It
  also needs no engine: there is no requirement to apply and no content
  to reconcile - only a default scaffold and a key-sort. The crate is
  `g3rs-json-parser` in `guardrail3/packages/v2/rs/parsers/`.

## Known stale doc (not touched this commit)

- `file-types/README.md` `json/` row still points `G3rsConfig` at this
  repo. It moved to guardrail3. Left as-is per scope.

## Verification

`cargo build --workspace` green after deletion (placeholders were never
workspace members, so no build impact).
