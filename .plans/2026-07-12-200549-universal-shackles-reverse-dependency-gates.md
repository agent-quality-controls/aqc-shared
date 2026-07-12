# Universal Shackles Reverse Dependency Gates

## Goal

- Keep every AQC workspace independent of all Shackles product crates after shared crate renames.

## Approach

- Replace obsolete `shakrs-json-parser` and `shakrs-runner` deny identities with `shackles-json-parser` and `shackles-runner`.
- Add `shackles-cli-support` to every AQC workspace deny list.
- Change no AQC manifests, source, runtime behavior, or public APIs.

## Decision

- Cargo-deny metadata belongs in each independent AQC workspace because reverse dependencies must fail at the package boundary.
- No compatibility entries are retained for removed crate identities.

## Files

- Every AQC package `deny.toml` that enforces the reverse dependency boundary.
