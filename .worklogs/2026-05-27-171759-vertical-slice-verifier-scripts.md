# Vertical-slice verifier scripts

## Summary

Companion verifier scripts for the clippy vertical-slice manifest. The
manifest itself lives in guardrail3 at
`.plans/g3v2-architecture/2026-05-26-191126-clippy-vertical-slice.md.manifest.toml`
(see guardrail3 worklog `2026-05-27-171759-vertical-slice-manifest.md`).

These scripts read that manifest and verify the aqc-shared code state
against it. Currently every layer fails (no code yet); they will turn
green incrementally as the engines + shared core are implemented.

## Files

- `scripts/_verify_lib.py` - shared helpers: manifest loader, source
  scanner, enum-body / struct-body parsers, cargo-deps reader,
  command runner.
- `scripts/verify-layer-1.sh` - tree existence.
- `scripts/verify-layer-2.sh` - public API (pub struct/enum/trait/fn
  declarations) existence.
- `scripts/verify-layer-3.sh` - enum variants exact match.
- `scripts/verify-layer-4.sh` - allowed_deps + forbidden_dep +
  forbidden_import.
- `scripts/verify-layer-5.sh` - struct field shapes + impl_required +
  trait_sig substrings + cargo build/clippy gates.
- `scripts/verify-all.sh` - runs all five and aggregates.

## Decisions

- **Python, not pure bash.** TOML parsing and structured source scans
  are awful in bash. Python 3.11+ tomllib handles the manifest;
  regex-based scanners handle Rust source. Trade-off: less rigorous
  than AST (syn-based) parsing; upgrade path noted.
- **Manifest path defaults to `../guardrail3/.plans/...`** relative to
  aqc-shared root. Override via `MANIFEST` env var.
- **Missing crates fail loudly.** First pass had layer 4 silently
  passing when no Cargo.toml existed (empty deps = subset of allow).
  Fixed.
- **Missing workspace root fails layer 5 verification_command rather
  than crashing.** First pass crashed with FileNotFoundError. Fixed.

## Verification

Ran `./scripts/verify-all.sh` against current aqc-shared state:

- Layer 1: 6/6 FAIL (all tree paths missing - crates not yet built).
- Layer 2: all FAIL (no source to grep).
- Layer 3: all FAIL (no enums to inspect).
- Layer 4: all FAIL with structured "cannot verify, Cargo.toml missing"
  messages (correct - not vacuous pass).
- Layer 5: all FAIL with concrete missing-struct / missing-impl /
  no-workspace messages.

Overall exit 1. Expected initial state.

## Verifier review summary

Walked through each layer against the user's stated priorities (file
tree, module relationships, types and their fields):

- **File tree** - layer 1. 6 paths.
- **Module relationships** - layer 4. Allow-list + forbidden pairs +
  forbidden imports across all three crates.
- **Types and fields** - layers 2, 3, 5.
  - Layer 2: type existence.
  - Layer 3: enum variants exact.
  - Layer 5: struct fields exact (name + type), trait impls present,
    trait method signatures substring-checked, cargo build + clippy
    gates.

The user's emphasis ("file tree, module relationships, types") is
covered. The grep-level field-and-signature checks are less rigorous
than full AST parsing; the documented upgrade path is `syn`-based
parsing if drift becomes a problem in practice.

## Key files

- `scripts/_verify_lib.py`
- `scripts/verify-layer-{1..5}.sh`
- `scripts/verify-all.sh`

Manifest reference:
- `~/Projects/agent-quality-controls/guardrail3/.plans/g3v2-architecture/2026-05-26-191126-clippy-vertical-slice.md.manifest.toml`

## Next steps

Implementation work in aqc-shared:

1. `packages/aqc-file-engine-core/` - framework types and FileEngine
   trait. Make layers 1, 2, 3 (partial), 4 (forbidden imports), 5
   (struct shapes, trait sig) pass for this crate.
2. `packages/file-types/toml/aqc-cargo-toml-engine/` - engine impl.
3. `packages/file-types/toml/aqc-clippy-toml-engine/` - engine impl.
4. Each commit should move at least one verifier row from FAIL to PASS.
   Final state: `./scripts/verify-all.sh` exits 0.
