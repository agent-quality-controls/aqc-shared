# FileTree and Waiver Cleanup

## Goal

Make the current g3rs workspace warnings reflect intended architecture.

## Approach

- Change `FileTree` so callers cannot mutate `entries` and break the sorted
  invariant used by `entry()`.
- Move recovery matching behavior out of `RecoveryRules`, leaving it as plain
  walk configuration data.
- Add waivers for source-shaped and requirement-shaped structs where named
  public fields are the intended API.
- Add a waiver for the large Rustfmt schema enum.

## Decisions

- `FileTree` should be fixed because it has a private invariant.
- `RecoveryRules` should be fixed by moving behavior, not by hiding fields.
- Dependency, feature, lint, profile, and glob records should be waived because
  they model file or requirement data.
- Import-count warnings stay as warnings.

## Files To Modify

- `packages/aqc-filetree/src/tree.rs`
- `packages/aqc-filetree/src/walk.rs`
- `packages/aqc-filetree/src/options.rs`
- `packages/aqc-filetree/tests/walk_tests.rs`
- `guardrail3-rs.toml`

## Verification

- `cargo test -p aqc-filetree`
- `cargo clippy -p aqc-filetree --all-targets --all-features -- -D warnings`
- `g3rs validate workspace --path .`
