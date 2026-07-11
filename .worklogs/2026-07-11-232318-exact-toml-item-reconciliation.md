# Summary

Fixed generic TOML exact-item reconciliation so exact arrays and array tables create missing members and repair mismatched member values. Released surface version is `aqc-toml-engine-core` 0.5.1 over `aqc-file-engine-core` 0.5.2.

# Decisions made

- Treat exact members as required members during reconciliation, while retaining exact removal of extras.
- Keep explicit `required` entries authoritative when the same identity also appears in `exact`.
- Fix the generic TOML primitives so every TOML engine receives the same behavior.
- Add contract tests for absent and mismatched array and array-table members.

# Key files for context

- `packages/file-types/toml/aqc-toml-engine-core/src/items/array.rs`
- `packages/file-types/toml/aqc-toml-engine-core/src/items/array_table.rs`
- `packages/file-types/toml/aqc-toml-engine-core/tests/core_contract.rs`
- `.plans/2026-07-11-213527-create-only-init-and-exact-items.md` in the Shackles repository

# Next steps

- Publish `aqc-toml-engine-core` 0.5.1.
- Update and release dependent AQC engines.
- Complete the atomic create-only init and executable verification work in Shackles.
