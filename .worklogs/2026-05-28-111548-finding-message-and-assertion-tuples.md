# Finding gets `message`; assertion entries gain (value, message) tuples

## Summary

Threaded a policy-authored `message` from the assertion entry through
the engine into `Finding::Mismatch`. Each lint, threshold, bool, string-
enum, msrv variant, and ban entry now carries an inline `message`. The
engine populates the matching finding field. Renderers (the spike, future
runner) surface it as the "why" line.

Engine-id constants exposed: `aqc_cargo_toml_engine::ENGINE_ID` and
`aqc_clippy_toml_engine::ENGINE_ID`. Consumers (the spike's registry +
path map) use the const; the impl returns it. Compile-time enforced.

## Decisions made

- One `message: String` (not `reason` + `fix_hint`). Policy decides how
  to structure the text. Validator is read-only -- text cannot assume
  auto-fix.
- Tuples, not wrapper types, for `(value, message)` in assertion data.
  Per-domain entry types (`BanEntry`) keep the same pattern: `path` plus
  `message` (rename of `reason`, mandatory now).
- `Finding::Mismatch` keeps `expected: String` and `current: Option<String>`
  as-is. Typing was never asked for; the engine pre-formats via the toml
  display impl, which loses nothing the renderer needs today.
- `ENGINE_ID` is a `pub const` exported from each engine crate. Impl reads
  it. Spike imports `ENGINE_ID as CARGO_ENGINE_ID` / `ENGINE_ID as
  CLIPPY_ENGINE_ID` and uses the consts in both `path_for` and the
  registry. Eliminates the prior triplicated string literal.
- Assertion-shape break:
  - `LintLevelsAssertion::Contains/IsExactly`: value is now `(String, String)`.
  - `LintLevelsAssertion::Excludes`: from `BTreeSet<String>` to
    `BTreeMap<String, String>` (name -> message).
  - `ThresholdsAssertion::Equals/AtMost/AtLeast`: value is now `(u64, String)`.
  - `ThresholdsAssertion::Present/Absent`: from `BTreeSet<String>` to
    `BTreeMap<String, String>`.
  - `MsrvAssertion::Equals/AtLeast`: variant tuple is now `(String, String)`.
  - `MsrvAssertion::OneOf`: variant tuple is now `(BTreeSet<String>, String)`.
  - `MsrvAssertion::Present/Absent`: variant tuple is now `(String)`.
  - `BoolAssertion::Equals`: variant tuple is now `(bool, String)`.
  - `BoolAssertion::Present/Absent`: variant tuple is now `(String)`.
  - `StringAssertion::Equals`: variant tuple is now `(String, String)`.
  - `StringAssertion::OneOf`: variant tuple is now `(BTreeSet<String>, String)`.
  - `StringAssertion::Present/Absent`: variant tuple is now `(String)`.
  - `BansAssertion::Excludes`: from `BTreeSet<String>` to
    `BTreeMap<String, String>` (path -> message).
  - `BanEntry`: `reason: Option<String>` -> `message: String` (mandatory).
- Reconcile sites that don't yet have a policy message (cargo's profile/
  package/dependency/feature assertions which the clippy adapter doesn't
  exercise) emit `message: String::new()` until those policy paths land.

## Key files for context

- `packages/aqc-file-engine-core/src/finding.rs` -- `Finding::Mismatch`
  gains `message: String`
- `packages/file-types/toml/aqc-{cargo,clippy}-toml-engine/src/lib.rs`
  -- `pub const ENGINE_ID: &str = "..."`
- `packages/file-types/toml/aqc-{cargo,clippy}-toml-engine/src/requirement.rs`
  -- assertion variant shapes updated
- `packages/file-types/toml/aqc-{cargo,clippy}-toml-engine/src/reconcile/*`
  -- match arms thread `message` to Mismatch
- `.plans/g3v2-architecture/2026-05-26-191126-clippy-vertical-slice.md.manifest.toml`
  -- BanEntry field name updated to `message`

## Next steps

- Engine doesn't yet recognise the inline-table form of group lints
  (`{ level = "deny", priority = -1 }`). Re-running v2 spike against the
  legacy `cargo-R00-clean-golden` fixture currently reports those 4 as
  Mismatch with `current = <absent>` because `as_str` returns None. Either:
  a) extend the engine to read inline-table lint entries; or
  b) extend `LintLevelsAssertion` to carry an optional priority and write
     the inline-table form.
- v2 strict policy requires 9 lints that legacy lists in
  `EXPECTED_CLIPPY_ALLOW` (legacy says these MAY be allowed). Document
  the divergence in the v1-v2 diff plan or pull baseline back to legacy
  parity -- decision pending.

## Verification

- `cargo build --workspace` -- PASS (aqc-shared and g3-v2-rs)
- `cargo clippy --workspace --all-targets -- -D warnings` -- PASS (both)
- `cargo fmt --all -- --check` -- PASS (both)
- `cargo test -p g3-clippy-linter-adapter --tests` -- 4 conflict-detection
  tests PASS (same-value no-conflict, disagreeing lint level emits
  PolicyConflict, msrv conflict, matching msrv no-conflict)
- `scripts/verify-all.sh` (both) -- ALL LAYERS PASS
- v2 spike against legacy `cargo-R00-clean-golden`: 33 of 45 lints reported
  as mismatches -- 4 due to inline-table form (priority handling missing),
  9 due to v2 being stricter than legacy R00, ~20 reasons under investigation
  (key naming difference may exist elsewhere too).
