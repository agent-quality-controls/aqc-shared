# aqc-cargo-toml-engine

`FileEngine` implementation for `Cargo.toml`.

This crate validates and reconciles Cargo manifest requirements, including:

- package and workspace package fields.
- Cargo lint table requirements.
- dependency presence and absence by package identity or local key.
- forbidden dependency package globs.
- features, profiles, target tables, workspace dependencies, and patch tables.

It uses `toml_edit` so reconciliation can update manifests while preserving
the rest of the document.
