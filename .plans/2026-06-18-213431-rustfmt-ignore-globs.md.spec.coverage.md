# Rustfmt Ignore Globs Spec Coverage

## Goal

- Covered by shared engine requirement and reconcile content checks.
- Covered by Rust merge and reconcile tests.

## Approach

- Covered by content checks for `RustfmtIgnorePathGlob`, shared
  `resolve_forbidden_globs`, reconcile application, invalid requirement
  reporting, and test names.

## Decisions

- Covered by constrained content checks: only `ignore` is wired.
- Negative choices are reviewed by reading the field names and tests.

## Files To Modify

- Covered by tree requirements.

## Verification

- Covered by `specular verify` for static checks.
- Covered by cargo test and targeted fmt for runtime and formatting checks.
