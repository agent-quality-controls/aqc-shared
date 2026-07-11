# Public value JSON schemas

## Summary

Added JSON Schema implementations to upstream public value types reused by downstream policy configuration. This prevents policies from duplicating version, toolchain-channel, and profile wire shapes.

## Decisions made

- `DottedVersion` derives its schema beside its transparent Serde representation.
- `RustToolchainChannel` derives a non-empty string schema beside its validation and serialization.
- `RustToolchainProfile` derives its exact lowercase enum schema beside its serialization.
- No file-engine reconciliation or file-format behavior changed.

## Key files for context

- `packages/aqc-file-engine-core/src/types.rs`
- `packages/file-types/toml/aqc-rust-toolchain-toml-engine/src/requirement/model.rs`
- `/Users/tartakovsky/Projects/agent-quality-controls/shackles/.plans/2026-07-11-181203-generated-cli-help.md`

## Next steps

- Release the changed crates in dependency order and consume the schemas through existing adapter facades.
