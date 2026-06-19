# FileTree and Waiver Cleanup

## Summary

Cleaned up the remaining current g3rs workspace warning set. `FileTree` now
keeps its sorted-entry invariant behind read-only accessors, and
`RecoveryRules` is plain configuration data with phase-2 matching owned by the
walker.

## Decisions Made

- Fixed `FileTree` instead of waiving it because `entry()` depends on sorted
  entries and public mutable fields could break that invariant.
- Moved recovery matching into `walk.rs` because recovery matching is phase-2
  walk behavior, not behavior of the public config record.
- Added waivers for Cargo dependency, feature, lint, profile, and glob records
  where named fields are the intended file-shaped or requirement-shaped API.
- Added large-type waivers for `ResolvedCargoTomlRequirements` and
  `RustfmtScalarSetting` because those shapes mirror Cargo sections and
  Rustfmt schema keys.
- Left import-count findings as warning-level pressure signals.

## Key Files

- `.plans/2026-06-19-110458-filetree-waiver-cleanup.md`
- `guardrail3-rs.toml`
- `packages/aqc-filetree/src/tree.rs`
- `packages/aqc-filetree/src/walk.rs`
- `packages/aqc-filetree/src/options.rs`
- `packages/aqc-filetree/tests/walk_tests.rs`

## Verification

- `cargo fmt -p aqc-filetree --check`
- `cargo test -p aqc-filetree`
- `cargo clippy -p aqc-filetree --all-targets --all-features -- -D warnings`
- `g3rs validate workspace --path .`
- `git diff --check`

## Next Steps

- Import-count warnings remain as refactor pressure. They should be addressed
  when the affected modules are already being split for cohesion.
