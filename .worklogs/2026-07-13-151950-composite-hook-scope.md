# Summary

Added the composite pre-commit hook to the exact Task 1 support-file allowlist.

# Decisions made

- Treated the composite hook as an exact gate-support file allowed by the plan.
- Did not broaden the allowlist to the `.githooks` directory.

# Key files for context

- `.githooks/pre-commit`
- `.githooks/pre-commit.d/shakrs`
- `specs/resolution-contract-cleanup.spec.json`

# Next steps

- Re-run Specular verification from the committed state.
