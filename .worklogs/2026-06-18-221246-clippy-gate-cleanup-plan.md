Summary:
- Added a plan for the remaining clippy gate cleanup after structural g3rs errors were cleared.

Decisions made:
- Keep the workspace clippy lint policy unchanged.
- Fix concrete issues directly and use localized reasoned `#[expect(...)]` only when the lint conflicts with intended data/API shapes.

Verification:
- `g3rs validate workspace --path . 2>&1 | rg '^\[Error\]|cargo gate failed'` reports only the cargo clippy gate.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` was run to inspect the failure categories.

Key files for context:
- `.plans/2026-06-18-221229-clippy-gate-cleanup.md`
- `Cargo.toml`
- `clippy.toml`

Next steps:
- Start with small concrete clippy fixes in the TOML engines, then rerun clippy per package.
