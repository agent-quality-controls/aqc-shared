# Final Registry Locks

## Summary

Regenerated every AQC lockfile from crates.io after the complete patch chain was published.

## Decisions Made

- Removed all local and unused patch records left by pre-fix release verification.
- Kept each independent workspace locked to dependencies compatible with Rust 1.85.

## Key Files

- `packages/**/Cargo.lock`

## Next Steps

- Use registry-only locks as the baseline for the next AQC change.
