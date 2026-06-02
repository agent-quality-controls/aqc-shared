# Engine erased contract trait

## Summary

Added the object-safe `Engine` contract to `aqc-file-engine-core`, beside the
typed `FileEngine`. It is the surface the g3-runner registry dispatches over
as `Box<dyn Engine>`: each engine knows the file it owns and reconciles the
type-erased requirements routed to it.

## Changes

- `aqc-file-engine-core/src/engine.rs`: `pub trait Engine { id(); target_path(&self, &Path) -> PathBuf;
  reconcile(&self, Option<&[u8]>, &[Box<dyn EngineRequirement>]) -> EngineOutput }`.
  Exported from lib.
- `aqc-cargo-toml-engine` + `aqc-clippy-toml-engine`: `impl Engine`. Each
  downcasts the erased reqs to its concrete `Req`, and for the v1 cardinality
  (one adapter -> one req per engine) calls the typed `FileEngine::reconcile`.
  More than one req surfaces an `InternalError` (the multi-adapter merge is a
  later slice; v1 routes a single adapter per engine).

## Notes

- `type_complexity` fires on the trait declaration's `&[Box<dyn EngineRequirement>]`
  only (not the impls); `#[expect]` sits on the trait method.
- No `Finding` change here. The runner derives a finding's `subject` from the
  engine's `target_path`; the tool-namespaced `rule` string is a report-layer
  concern handled in the runner.

## Verification

`cargo build/clippy -D warnings/test` green; `verify-all.sh` ALL LAYERS PASS;
`cargo dupes` clean.
