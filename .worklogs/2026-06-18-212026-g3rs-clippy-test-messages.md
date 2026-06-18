## Summary

Cleared old g3rs Clippy TOML weak test-message findings and added the resolved aggregate public-field waiver.

## Decisions Made

- Replaced weak `"utf8"`, `"msrv"`, and `"equals mismatch"` test messages with specific failure descriptions.
- Added a waiver for `ResolvedClippyTomlRequirements` because it is a resolved data aggregate mirroring the input requirement sections.
- Left structural Clippy TOML source splitting for a later batch.

## Verification

- `rg 'expect\("utf8"\)|expect\("msrv"\)|expect\("equals mismatch"\)' packages/file-types/toml/aqc-clippy-toml-engine/tests -n`
- `cargo fmt -p aqc-clippy-toml-engine`
- `cargo test -p aqc-clippy-toml-engine`
- `g3rs validate workspace --path .` still fails on remaining large-file/import-count and workspace clippy findings. The Clippy TOML weak test-message and resolved aggregate public-field errors are gone.

## Key Files For Context

- `packages/file-types/toml/aqc-clippy-toml-engine/tests/merge.rs`
- `guardrail3-rs.toml`

## Next Steps

- Split `packages/file-types/toml/aqc-clippy-toml-engine/src/requirement.rs`.
- Reduce imports in that file.
- Split remaining large Cargo engine files.
