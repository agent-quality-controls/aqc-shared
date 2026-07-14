# Summary

Aligned `aqc-toml-engine-core` with the strict Clippy policy by forbidding `std::process::abort`.

# Decisions Made

- Updated managed Clippy configuration instead of waiving a missing product requirement.
- Kept concrete-engine configuration changes with their dependent engine commits.

# Key Files For Context

- `packages/file-types/toml/aqc-toml-engine-core/clippy.toml`

# Next Steps

- Align the concrete TOML engines while committing their `0.7.0` changes.
