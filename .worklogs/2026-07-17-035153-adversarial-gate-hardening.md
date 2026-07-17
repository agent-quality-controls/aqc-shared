# Adversarial Gate Hardening

## Summary

Closed the final adversarial bypasses in architecture discovery, staged Cargo dependency selection, staged Shakrs validation, and reusable pre-push configuration caches.

## Decisions Made

- Generic `tests` and `specs` directory names do not exclude nested production crates; only repository support roots, generated directories, and the architecture checker's own fixture tree are excluded.
- G3RS staged checks materialize the captured immutable Git tree through a private index and derive local registry patches from that snapshot, including transitive local dependencies.
- Shakrs pre-commit validation materializes the staged index in a detached worktree and preserves an inherited alternate index path.
- Pre-push retains generated Cargo homes so unchanged configuration keeps stable inode and modification metadata.
- Executable Python regressions cover atomic concurrent generation, unchanged metadata, explicit path exclusion, immutable tree extraction under index mutation, and an offline Cargo compile over staged-only package source.

## Key Files For Context

- `tools/aqc-requirement-architecture/src/fs.rs`
- `scripts/local_cargo_source.py`
- `scripts/test-local-cargo-source.py`
- `.githooks/bin/g3rs`
- `.githooks/pre-commit.d/shakrs`
- `.githooks/pre-push`

## Next Steps

- Keep the permanent Specular contract and full local gate passing when discovery or hook routing changes.
