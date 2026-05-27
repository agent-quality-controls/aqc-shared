# Drop `aqc-toml-parser`; reuse upstream crates directly

## Summary

Removed the `aqc-toml-parser` stub. The three-layer
(`format -> domain -> engine`) pattern collapses to two layers
(`domain -> engine`) for TOML because `toml_edit` is already the canonical
TOML editor in the Rust ecosystem - wrapping it would be pure indirection.

Same conclusion for `serde_json` and `jsonc-parser`: don't wrap mainstream
typed crates. Reuse them.

For Cargo specifically: `cargo-util-schemas` (maintained inside
`rust-lang/cargo`) ships the exact typed schema cargo itself parses with.
The `aqc-cargo-toml-parser` package will re-export it rather than
hand-rolling or generating types.

## Decisions made

- **No `aqc-toml-parser` crate.** Domain parsers (`aqc-clippy-toml-parser`,
  `aqc-cargo-toml-parser`, ...) and domain engines depend on `toml_edit`
  and `toml` directly. Reduces the package count and stops us inventing
  a surface to maintain.
- **Two patterns for domain parser packages:**
  - **Pattern A** (upstream schema exists, e.g. `Cargo.toml` →
    `cargo-util-schemas`): single crate, re-export upstream types,
    no generator, no verifier.
  - **Pattern B** (no upstream schema, e.g. `clippy.toml`): generator
    crate + verifier, like the existing `aqc-clippy-toml-parser`.
- **Engines are uniform regardless of pattern**: `reconcile(bytes, reqs) -> bytes`
  via `toml_edit` directly.
- **Cargo.toml schema source = `cargo-util-schemas`.** Legacy guardrail3
  hand-rolled 430 lines because the crate didn't exist then; we don't
  repeat that.
- **The README at `packages/file-types/` now lists "schema source per
  domain"** instead of claiming there is a format-layer crate.

## Verification

- Stub `packages/file-types/toml/aqc-toml-parser/.gitkeep` deleted.
- `packages/file-types/README.md` updated to drop the
  `aqc-{format}-parser` middle step.
- Architecture plan edits in `guardrail3/.plans/g3v2-architecture/` (the
  one repo that holds plans across both products; plan edits there are
  explicitly allowed):
  - `2026-05-21-195830-repo-workspace-plugin-generation-model.md`:
    rewrote the "Shared `aqc-*` packages" and "Config file path -> package
    map" sections to drop layer 1.
  - `2026-05-26-193045-aqc-parser-migration.md`: full rewrite. Two
    patterns (A: upstream re-export, B: generator). Schema-source table
    per file. Build order. `cargo-util-schemas` declared for Cargo.

## Key files for context

- `~/Projects/agent-quality-controls/guardrail3/.plans/g3v2-architecture/2026-05-21-195830-repo-workspace-plugin-generation-model.md` -
  canonical architecture; updated sections near L227-L290 and L520-L580.
- `~/Projects/agent-quality-controls/guardrail3/.plans/g3v2-architecture/2026-05-26-193045-aqc-parser-migration.md` -
  rewritten end-to-end.
- `packages/file-types/README.md` - updated convention table.

## Why not just keep the stub

A stub directory with a `.gitkeep` is a TODO disguised as architecture.
It tells future contributors "there should be a thing here," which is
wrong - there should NOT be a thing there. Deleting it removes the
ambient pressure to build a crate we don't need.

## Next steps

1. **`aqc-cargo-toml-parser`** (Pattern A): tiny crate that re-exports
   `cargo-util-schemas::manifest::*`. Smoke test parses a real
   `Cargo.toml` and reads `manifest.lints`.
2. **`aqc-clippy-toml-engine`** (uniform): `reconcile` for `clippy.toml`
   via `toml_edit`.
3. **`aqc-cargo-toml-engine`** (uniform): `reconcile` for
   `Cargo.toml`'s `[lints.clippy]` table via `toml_edit`.
4. Wire the four into the clippy vertical slice.

Same downstream cutover and CI deferrals as previous worklogs.
