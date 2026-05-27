# EngineRequirement trait + per-engine impls

## Summary

Added object-safe `EngineRequirement` trait to `aqc-file-engine-core`.
Engine-specific `Req` types (`CargoTomlRequirement`, `ClippyTomlRequirement`)
now impl it. This is the abstraction adapters use to return
`Vec<Box<dyn EngineRequirement>>` and the broker uses to dispatch by
`engine_id` + downcast via `as_any`.

## Decisions made

- Trait, not closed enum. Reason: ~20 engines planned -- a closed enum
  in a central crate would force every adapter to transitively depend on
  every engine crate. Trait inverts the dependency: each engine impls it
  for its own type; each adapter depends only on the engines it targets.
- Trait lives in `aqc-file-engine-core`, not in a new
  `aqc-engine-requirement` crate. Engines already depend on core; the
  trait does not depend on any engine type, so there's no cycle.
- `engine_id()` returns the crate name verbatim. The broker's registry
  is keyed by this string.
- `Any` is the downcast mechanism. Project's clippy config bans
  `std::any::Any` globally for type-erasure reasons; added file-scoped
  `#[expect(clippy::disallowed_types, reason = "...")]` at exactly the
  three sites that need it. No global allow.

## Key files for context

- `packages/aqc-file-engine-core/src/requirement.rs` -- the trait
- `packages/aqc-file-engine-core/src/lib.rs` -- re-export
- `packages/file-types/toml/aqc-cargo-toml-engine/src/requirement.rs`
  -- impl + tests
- `packages/file-types/toml/aqc-clippy-toml-engine/src/requirement.rs`
  -- impl + tests

## Next steps

Create new cargo workspace at
`guardrail3/packages/v2/rs/`; build `aqc-clippy-linter-adapter`,
`g3-clippy-strict`, and `g3-runner-spike` against this trait. See
`guardrail3/.plans/g3v2-architecture/2026-05-27-185042-clippy-policy-and-adapter-slice.md`.

## Verification

- `cargo build --workspace` -- PASS
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` -- PASS
- `cargo test -p aqc-cargo-toml-engine -p aqc-clippy-toml-engine` -- PASS
- `scripts/verify-all.sh` -- ALL LAYERS PASS
