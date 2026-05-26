# aqc-clippy-toml-parser scaffolding + repo guardrails

## Summary

Stood up the first real package in aqc-shared - `aqc-clippy-toml-parser` - as
a generator-driven typed parser for `clippy.toml`. Also wired up the repo-level
g3rs hooks at the aqc-shared root.

The schema is no longer hand-maintained: a generator binary downloads the
pinned upstream `conf.rs` from `rust-lang/rust-clippy@rust-1.95.0`, parses the
`define_Conf!` macro, and emits the typed struct into the sibling types crate.
The pin lives in two places (manifest.toml + a const in the generator) and a
verifier enforces they agree.

## Decisions made

- **Package layout = 3-crate facade.** Top-level Cargo.toml is both
  `[package]` and `[workspace]`, mirroring the legacy
  `guardrail3/packages/parsers/clippy-toml-parser/` shape. Member crates:
  `crates/types/` (generated schema), `crates/runtime/` (read/write API,
  currently a stub re-export), `crates/generator/` (binary). Top-level
  `src/lib.rs` and `src/types.rs` are facade re-exports only. Each `lib.rs`
  is exports-only; real code lives in named sibling files.
- **Schema is generated, not hand-written.** Legacy parsers were
  hand-maintained ~140-field structs. The new pattern is: pin an upstream
  source, generate the schema, verify mechanically. Trades 0 maintenance
  per upstream release for a one-line tag bump.
- **Pinned to `rust-1.95.0`** (not master). The repo toolchain is `stable`
  so we pick the latest tagged release of clippy. Bumping the pin is a
  deliberate two-line change (const + manifest) followed by rerunning the
  generator; verification layer 3 enforces those two declarations agree.
- **HTTP via `ureq` instead of `reqwest`** to keep the dep tree light.
- **Generated file ships its own `#![allow(...)]`** for the lints a schema
  mirror inherently fails (`derive_partial_eq_without_eq`,
  `struct_excessive_bools` with 97 bool-bearing fields, `type_complexity`
  for nested tuple types, `too_many_lines` for the long `Default` impl,
  `str_to_string`). Reason: don't push schema-shape friction onto consumers.
- **Generator binary gets a relaxed lint set** via crate-root `#![allow]`
  in `crates/generator/src/main.rs`. The strict library lints (`unwrap_used`,
  `print_stdout`, `disallowed_methods` for `std::fs`) fight the shape of a
  one-shot CLI; the relaxation is explicit and localized.
- **`std::fs` access is funneled through `crates/generator/src/fs.rs`** so
  the g3rs `direct-fs-usage` finding hits exactly one file by design.
- **No CI yet.** g3rs reports 3 release-related warnings (release-plz
  workflow, publish-dry-run, CARGO_REGISTRY_TOKEN). Deferred per user.
- **No edits to legacy guardrail3.** Confirmed frozen, used as
  read-only reference for clippy.toml, rustfmt.toml, deny.toml,
  rust-toolchain.toml, workspace.lints baseline, and hook layout.

## Verification

- `g3rs validate repo` (aqc-shared root): no findings.
- `g3rs validate workspace` (parser package): 1 waived warning
  (`large-type-inventory` on `ClippyToml`, waived in `guardrail3-rs.toml`)
  and 3 deferred release/CI warnings.
- `./verify/verify-all.sh`: all 5 layers green (tree, public API,
  generator contract, generated code contract, compilation).
- `cargo build --workspace`: all 4 crates compile clean.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`:
  clean.
- `cargo fmt --all -- --check`: clean.

## Key files for context

Cold-start reading list for whoever picks this up next:

- `packages/file-types/toml/aqc-clippy-toml-parser/manifest.toml` - the
  authoritative spec the verifier checks against. Bump the pin here AND in
  the generator const together.
- `packages/file-types/toml/aqc-clippy-toml-parser/crates/generator/src/main.rs`
  - `CLIPPY_TAG`, `OUTPUT_RELATIVE`, `CACHE_RELATIVE` constants.
- `packages/file-types/toml/aqc-clippy-toml-parser/crates/generator/src/render.rs`
  - the file-level `#![allow]` block that gets emitted into the generated
  types file. Add new lints here when upstream changes break new rules.
- `packages/file-types/toml/aqc-clippy-toml-parser/crates/generator/src/parse.rs`
  - the `define_Conf!` line-by-line parser. Fragile to upstream macro shape
  changes; will need a rewrite if Clippy restructures `conf.rs`.
- `packages/file-types/toml/aqc-clippy-toml-parser/guardrail3-rs.toml` -
  per-workspace g3rs policy: allowed_deps, waivers.
- `packages/file-types/toml/aqc-clippy-toml-parser/verify/verify-layer-{1..5}.sh`
  - mechanical verification. Layer 3 is load-bearing: it ties manifest pin
  to generator const to generated-file header.
- `.githooks/pre-commit` + `.githooks/pre-commit.d/g3rs` - repo-level
  hooks emitted by `g3rs init repo`. `core.hooksPath=.githooks`.

## Next steps

1. **`aqc-clippy-toml-engine`** - implement `reconcile(bytes, requirements) -> bytes`
   using `toml_edit` to preserve formatting. Required by the clippy vertical
   slice. Stub directory already exists under
   `packages/file-types/toml/aqc-clippy-toml-engine/`.
2. **`aqc-cargo-toml-parser`** - same generator pattern. Cargo's schema
   does not have a `define_Conf!`-like macro; pick a source-of-truth approach
   (`cargo-manifest` crate? scrape the manifest reference page? hand-write
   with a diff harness?) before building.
3. **`aqc-cargo-toml-engine`** - reconcile `[lints.clippy]` and other tables.
4. Once 1-3 are done, swap the guardrail3 vertical slice off the legacy
   hand-written parsers onto the aqc-shared packages.
5. Eventually: add CI (release-plz workflow, publish-dry-run, registry token)
   to clear the 3 deferred g3rs release warnings. Not blocking anything yet.
