# Summary

Removed `patch.unused` records introduced into independent-workspace lockfiles by a temporary repository-wide Cargo source override used during local verification.

# Decisions made

- Keep lockfiles source-shaped and independent of local development overrides.
- Use workspace-local temporary resolution only for commit-hook verification, then remove it.
- Do not publish crates or bypass commit hooks to resolve unpublished local versions.

# Key files for context

- `packages/**/Cargo.lock`
- `.githooks/pre-commit.d/g3rs`
- `.githooks/pre-commit.d/shakrs`

# Next steps

- Commit the Shackles TSC policy, adapter, CLI registration, specs, and fixtures.
