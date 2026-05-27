# Engine surface expansion + Finding type collapse

## Summary

Expanded `CargoTomlRequirement` and `ClippyTomlRequirement` from single-section
shapes (lints / msrv+method_bans+thresholds) to the full target list defined in
the manifest: 7 fields each, declarative bulk-form, struct-of-fields-per-target.
Collapsed the three prior error/conflict types (`EngineError`, `MergeConflict`,
`Finding`) into a single `Finding` enum with variants for every kind of
deviation an engine can report. Moved the shared `toml_edit` parse helper and
`parse_version_tuple` into `aqc-file-engine-core` so both engines reuse them.

## Decisions made

- `Finding` is one enum with 6 variants: `Mismatch`, `UnwritableRequiredKey`,
  `SchemaError`, `ParseError`, `PolicyConflict`, `InternalError`. Rejected
  keeping `EngineError` and `MergeConflict` separate -- they were three names
  for the same "thing the engine wants to surface to the runner."
- `FileEngine::reconcile` returns `EngineOutput` directly, not `Result`.
  Parse failures surface as `Finding::ParseError`, not `Err`.
- `parse_or_report` and `parse_version_tuple` live in `aqc-file-engine-core`,
  not in each engine. Reason: cargo/clippy engines both need them; otherwise
  they were duplicated. Added `toml_edit = "0.25"` as the first (and only)
  dep of `aqc-file-engine-core`. Manifest `allowed_deps` updated.
- `reconcile/` directory in both engines is split into per-section modules
  (lints / msrv / thresholds / bans / bools / enums / package_fields /
  workspace_lints / workspace_package_fields / profiles / dependencies /
  features) plus a `dispatch.rs` apply function. Facade `mod.rs` only re-exports.
- Clippy `disallowed-methods` ban for `toml_edit::de::*` paths marked
  `allow-invalid = true` (those functions are not reachable for clippy
  resolution; the ban still applies if a future engine tries to call them via
  some other path).
- Accepted by-design duplication (`apply_present`, `apply_absent`, `is_exactly_only`,
  `current_list`, wrapper `apply` functions in `workspace_*` and `bools`/`enums`)
  via 8 `cargo dupes ignore` entries in `.dupes-ignore.toml`. Each is a
  per-section variant of the same symmetric assertion shape -- factoring further
  would require generics over closures with 5+ shared params.

## Key files for context

- `packages/aqc-file-engine-core/src/finding.rs` -- the new collapsed `Finding` enum
- `packages/aqc-file-engine-core/src/toml_helpers.rs` -- `parse_or_report`,
  `parse_version_tuple`
- `packages/file-types/toml/aqc-cargo-toml-engine/src/requirement.rs` -- 7-field
  declarative requirement
- `packages/file-types/toml/aqc-clippy-toml-engine/src/requirement.rs` -- 7-field
  declarative requirement (unified `BansAssertion`/`BanEntry` for
  methods/types/macros)
- `packages/file-types/toml/*/src/reconcile/dispatch.rs` -- dispatcher
- `.dupes-ignore.toml` -- accepted-duplication list

## Next steps

1. Plan policy + linter-adapter layers carefully. Target dirs:
   `~/Projects/agent-quality-controls/guardrail3/packages/v2/rs/policies/` and
   `~/Projects/agent-quality-controls/guardrail3/packages/v2/rs/linter-adapters/`.
2. Build clippy linter adapter (produces `ClippyTomlRequirement` from policy
   inputs).
3. Build clippy policy (declares which lints to enable + their levels).
4. Wire them in an ad-hoc runner script (location TBD, placed "randomly" per
   user directive).

## Verification

- `cargo build --workspace` -- PASS
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` -- PASS
- `cargo fmt --all -- --check` -- PASS
- `cargo dupes check --min-lines 8 --max-exact 85 --max-exact-percent 10` -- PASS (0.0% exact)
- `scripts/verify-all.sh` -- ALL LAYERS PASS
- `g3rs validate workspace --path .` -- only legacy warns
  (`g3rs-release/*` workflow checks, `public-struct-named-fields` for waived
  declarative requirement structs)
