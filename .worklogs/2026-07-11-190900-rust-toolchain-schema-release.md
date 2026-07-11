# Rust toolchain schema release

## Summary

Prepared `aqc-rust-toolchain-toml-engine 0.4.2` with upstream-owned channel and profile schemas.

## Decisions made

- Depend on released `aqc-file-engine-core 0.4.2` so downstream schema derivation remains registry-only.

## Key files for context

- `packages/file-types/toml/aqc-rust-toolchain-toml-engine/src/requirement/model.rs`
- `.worklogs/2026-07-11-190617-public-value-json-schemas.md`

## Next steps

- Publish the engine and update its downstream adapter.
