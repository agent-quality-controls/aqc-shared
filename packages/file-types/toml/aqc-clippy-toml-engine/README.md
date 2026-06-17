# aqc-clippy-toml-engine

`FileEngine` implementation for `clippy.toml`.

This crate validates and reconciles Clippy configuration requirements,
including:

- `msrv`.
- numeric thresholds.
- boolean and string-valued settings.
- `disallowed-methods`, `disallowed-types`, and `disallowed-macros`.
- forbidden path globs for those disallowed lists.

It uses `toml_edit` so reconciliation can update `clippy.toml` while preserving
the rest of the document.
