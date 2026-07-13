# Summary

Corrected the Task 1 changed-scope verifier to recognize only the named package-local proof and gate support allowed by the plan.

# Decisions made

- Kept the frozen pre-Task baseline so committed drift remains visible.
- Listed each required support file explicitly instead of broadening directory prefixes.
- Preserved the exact managed Shackles hook snippet required by the hooks policy.

# Key files for context

- `specs/resolution-contract-cleanup.spec.json`
- `.githooks/pre-commit`
- `.githooks/pre-commit.d/shakrs`

# Next steps

- Re-run the full AQC Specular verification from the committed state.
- Complete Shackles command-gate and dependency-generation verification.
