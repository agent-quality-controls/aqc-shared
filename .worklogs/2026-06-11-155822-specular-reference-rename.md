# Specular reference rename

## Summary

Updated shared-library documentation and comments so cross-product references use Specular. No code behavior changed.

## Decisions made

- Kept Fixture3 and Guardrail3 names unchanged because those are separate products.
- Updated only documentation, comments, and prior worklog prose.

## Key files for context

- `README.md`
- `packages/aqc-filetree/plan.md`
- `packages/aqc-fs-utils/plan.md`
- `packages/aqc-fs-utils/src/lib.rs`
- `packages/aqc-git-helpers/plan.md`
- `packages/aqc-git-helpers/src/lib.rs`

## Verification

- Local stale-name scan across AQC checkouts.
- Tracked filename scan for previous project names.

## Next steps

- None.
