# Summary

Migrated the text, Cargo, Clippy, deny, rust-toolchain, and rustfmt engines to return either a complete resolved requirement root or every merge conflict. Removed conflict-block state and made resolved roots externally read-only through borrowed getters.

# Decisions made

- Preserved conflict collection order, keys, reasons, and provenance while preventing reconciliation of partial resolved state.
- Exposed optional resolved fields as `Option<&T>` and vectors as slices rather than lint-disallowed container references.
- Corrected the Specular checks so they distinguish deleted conflict-block fields from retained conflict-detection functions and accept immutable borrowed getter forms.
- Regenerated every lockfile against the published core generation so package checks cannot pass through local path resolution.

# Key files for context

- `packages/file-types/text/aqc-text-engine-core/src/requirement`
- `packages/file-types/toml/aqc-cargo-toml-engine/src/requirement`
- `packages/file-types/toml/aqc-clippy-toml-engine/src/requirement`
- `packages/file-types/toml/aqc-deny-toml-engine/src/requirement`
- `packages/file-types/toml/aqc-rust-toolchain-toml-engine/src/requirement`
- `packages/file-types/toml/aqc-rustfmt-toml-engine/src/requirement`
- `specs/resolution-contract-cleanup.spec.json`

# Next steps

- Commit and publish all six engine releases.
- Migrate the Shackles crate generation and verify runner command gating end to end.
