# Universal Shackles Reverse Dependency Gates

## Summary

Updated every AQC workspace Cargo-deny boundary for the universal Shackles parser, runner, and CLI-support package identities. No AQC code or manifests changed.

## Decisions made

- Removed obsolete family-named parser and runner identities without aliases.
- Kept reverse dependency enforcement local to each independent AQC workspace.

## Key files for context

- `.plans/2026-07-12-200549-universal-shackles-reverse-dependency-gates.md`
- `specs/universal-shackles-reverse-dependency-gates.spec.json`
- Package-local `deny.toml` files

## Next steps

- Publish the universal Shackles crates; no AQC release is required for metadata-only deny changes.
