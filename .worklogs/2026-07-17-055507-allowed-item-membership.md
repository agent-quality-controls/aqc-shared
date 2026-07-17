# Allowed Item Membership

## Summary

Added universal allowed item membership for optional identities in closed collections. Applied it across JSON, TOML, Cargo, Clippy, rust-toolchain, pnpm YAML, architecture checks, and permanent Specular verification.

## Decisions

- `allowed` restricts present identities without requiring or initializing them.
- Multiple allowed collections intersect.
- `exact` remains a complete required collection and takes precedence during reconciliation.
- Required baseline plus optional permitted identities is represented by `required + allowed`, not `exact + allowed`.
- Unexpected-item findings include only allowed or exact contributors that reject that specific file item.
- Identity conflicts are reported from raw constructive inputs even when value composition also fails.
- Concurrent local-source test cleanup is unconditional and bounded so a failed worker cannot leave the local gate waiting on its reader thread.

## Key Files

- `packages/aqc-file-engine-core/src/merge/model.rs`
- `packages/aqc-file-engine-core/src/merge/item_model.rs`
- `packages/aqc-file-engine-core/src/merge/items.rs`
- `packages/aqc-file-engine-core/src/merge/forbidden_globs.rs`
- `.plans/2026-07-17-051043-allowed-item-membership.md`
- `specs/explicit-setting-membership.spec.json`
- `specs/verifiers/verify-explicit-setting-membership.py`
- `scripts/test-local-cargo-source.py`

## Verification

- `specular lint specs/explicit-setting-membership.spec.json`
- `specular verify specs/explicit-setting-membership.spec.json`
- `scripts/check-workspaces.sh`
- Adversarial review converged after fixes for skipped allowed-only paths, attribution, failed-composition conflicts, malformed dependencies, duplicate classifications, nonconstructive toolchain requirements, path-mode fixed points, and permanent coverage.

## Next Steps

- Pin Shackles to this AQC revision.
- Complete the Deny policy migration with `required + allowed` table keys.
