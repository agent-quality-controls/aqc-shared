# aqc-deny-toml-engine

`FileEngine` implementation for `deny.toml`.

This crate validates and reconciles cargo-deny configuration requirements with
`toml_edit`, preserving unrelated TOML content while applying the requested
settings.

Managed sections include:

- `graph` targets, exclusions, features, and feature mode flags.
- `advisories` database, lint-level, scope, ignore, and staleness settings.
- `licenses` allow lists, exceptions, clarifications, private-source settings,
  and unused-entry lint levels.
- `bans` package allow/deny/skip entries, feature bans, workspace dependency
  checks, and build-script checks.
- `sources` registry, git, private source, and organization allow lists.

The public requirement surface mirrors cargo-deny's file schema. Shared merge,
list, item, scalar, conflict, and provenance behavior comes from
`aqc-file-engine-core`; TOML parsing and edit mechanics come from
`aqc-toml-engine-core`.
