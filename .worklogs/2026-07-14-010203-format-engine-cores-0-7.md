# Summary

Added reusable JSON and YAML engine cores, updated the TOML core to shared `0.7.0` primitives, and renamed the text crate to `aqc-text-file-engine` to match its concrete engine role.

# Decisions Made

- Kept JSON and YAML parsing and write mechanics in format-specific engine cores.
- Kept tool and product meaning out of every format core.
- Removed the misleading text `-core` name with no alias or compatibility crate.
- Preserved pure byte-to-byte engine behavior with no filesystem access.
- Regenerated the repository hook fragment so the staged text-engine workspace rename routes only the new workspace.

# Key Files For Context

- `packages/file-types/json/aqc-json-engine-core/src/lib.rs`
- `packages/file-types/yaml/aqc-yaml-engine-core/src/lib.rs`
- `packages/file-types/toml/aqc-toml-engine-core/src/scalars.rs`
- `packages/file-types/text/aqc-text-file-engine/src/lib.rs`
- `.githooks/pre-commit.d/shakrs`

# Next Steps

- Commit JSON, YAML, and TOML concrete engines against these format cores.
