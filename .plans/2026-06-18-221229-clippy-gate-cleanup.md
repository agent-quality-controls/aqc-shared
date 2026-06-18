Goal:
- Make `g3rs validate workspace --path .` pass by clearing the remaining cargo clippy gate failure.

Approach:
- Fix concrete behavior/readability issues directly: excessive nesting, shadowing, indexing in two-item windows, wildcard enum arms, similar local names, and module-name repetition where a local rename is clean.
- Use small type aliases where they clarify repeated requirement/result shapes and reduce `type_complexity`.
- Use reasoned `#[expect(...)]` only for generated or inherent-domain shapes where the lint conflicts with the design, such as large rustfmt setting inventories or unavoidable public data records.
- Avoid broad workspace-level lint downgrades.

Key decisions:
- Do not change `[workspace.lints.clippy]`; it is the repository standard.
- Do not add crate-level allows. If an exception is needed, put it close to the item and include a reason.
- Keep commits small by crate or lint family so regressions are easy to isolate.

Files likely to modify:
- `packages/file-types/toml/aqc-clippy-toml-engine/src/reconcile/bans.rs`
- `packages/file-types/toml/aqc-clippy-toml-engine/src/requirement/scalar.rs`
- `packages/file-types/toml/aqc-cargo-toml-engine/src/reconcile/dependencies/removals.rs`
- `packages/file-types/toml/aqc-cargo-toml-engine/src/requirement/*.rs`
- `packages/file-types/toml/aqc-rustfmt-toml-engine/src/requirement.rs`
- `packages/file-types/toml/aqc-rustfmt-toml-engine/src/reconcile/settings/*.rs`
