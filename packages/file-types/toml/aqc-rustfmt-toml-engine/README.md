# aqc-rustfmt-toml-engine

`FileEngine` implementation for `rustfmt.toml`.

This crate validates and reconciles Rustfmt configuration requirements,
including:

- scalar settings such as `edition`, `max_width`, and `reorder_imports`.
- list settings such as `ignore` and `skip_macro_invocations`.
- optional closed-setting checks for policies that own the full config.

It uses `toml_edit` so reconciliation can update `rustfmt.toml` while
preserving the rest of the document.
