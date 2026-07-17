# Adversarial Gate Hardening

## Summary

Closed the final adversarial bypasses in architecture discovery, staged Cargo dependency selection, staged Shakrs validation, and reusable pre-push configuration caches.

## Decisions Made

- Generic `tests`, `specs`, and `fixtures` directory names do not exclude nested production crates; only repository support roots, generated directories, and the architecture checker's own fixture tree are excluded.
- Tracked manifests below generated `target` directories are added back to architecture discovery and full workspace checks, so force-added production crates cannot hide behind build-output exclusions.
- The checker's own fixture tree remains excluded even when a fixture contains a force-added `target` manifest; this explicit exception takes precedence over tracked-target recovery.
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
