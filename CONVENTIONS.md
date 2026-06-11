# Crate conventions enforced by the gates (read before writing a crate)

The pre-commit chain runs the installed `g3rs` binary plus cargo gates. Most
of its structural rules exist ONLY compiled into that binary -- this file is
the written form, reverse-engineered from the rule sources and verified
against real commits (2026-06-07, the aqc-fs-utils / aqc-filetree /
aqc-git-helpers build, five rejected commits before this list existed).

A plan for a new crate must account for every row here, or its
implementation will not commit.

## Structure

- **`lib.rs` is facade-only.** Module declarations and single-item `pub use`
  lines, nothing else: no types, no fns, no consts, no macros. Same rule for
  any `mod.rs` (no grouped `pub use x::{a, b}` re-exports there either).
- **Every facade export is feature-gated.** `Cargo.toml` declares
  `[features] default = ["all"]; all = ["api"]; api = []` and every
  `pub use` in `lib.rs` carries `#[cfg(feature = "api")]`.
- **I/O lives in named boundary modules.** Direct `std::fs` is allowed only
  in `src/fs.rs` / `src/fs/mod.rs` (or a crate at `fs/src/lib.rs`); confine
  subprocess invocations the same way (e.g. `src/git.rs`) with a scoped
  `#![expect(clippy::disallowed_methods, reason = ...)]`. Everything else
  calls through the boundary module.
- **No per-crate `clippy.toml` / config files inside workspace members**
  (`workspace-local-file-placement`). Capability exceptions are declared
  `#[expect]`s in the boundary module or waivers in the repo
  `guardrail3-rs.toml` (waivers require `rule`, `subject`, `selector`,
  `reason`).

## Tests

- **Integration test files must be named `*_test.rs` or `*_tests.rs`** (or
  live under a top-level `tests/` segment of the SCANNED root -- in this
  repo the scan root is the workspace, so the filename suffix is what
  actually classifies them). Unclassified test files are linted as
  production code.
- **`.expect()` outside a `#[test]` fn is flagged even in test files**
  (clippy's `allow-expect-in-tests` covers `#[test]` fns, not helpers); use
  a file-level `#![expect(clippy::expect_used, reason = ...)]` when fixture
  helpers need it.
- **Expect/assert messages must be sentences** (`test-expect-message-quality`
  rejects fragments like "mkdir src"; write "fixture must create src/").

## Code shape (the strict-clippy workspace file)

- `type-complexity-threshold = 75`: even `Result<Vec<T>, E>` fires. Declare
  the shape with `#[expect(clippy::type_complexity, reason = ...)]` -- NEVER
  hide it behind an alias to silence the lint (taxonomy decision 2026-06-07;
  aliases are for meaning, not evasion).
- `too-many-lines-threshold = 75` per fn, `excessive-nesting-threshold = 4`,
  ≤ 20 imported names per file (`too-many-use-imports`; pure-facade files
  exempt), no wildcard enum arms, `missing_docs_in_private_items`.
- Public structs with named pub fields need either plain-data shape (no
  inherent methods) or a `guardrail3-rs.toml` waiver
  (`public-struct-named-fields`; it WARNS for shared crates that mix pub
  fields with methods).
- Error enums: typed variants, no `String`-only public error forms
  (`public-weak-error-forms`).

## Toolchain + release gates

- **MSRV 1.85 is verified** (`cargo msrv verify -- cargo check --locked`):
  no let-chains, no post-1.85 syntax.
- `cargo fmt --check`, `cargo dupes check --min-lines 8 --max-exact 85
  --max-exact-percent 10` (repo root), deny/dep-allowlist (new deps go into
  the repo `guardrail3-rs.toml` `allowed_deps` AND must pass `cargo deny`),
  gitleaks.
- New workspace members: add to the root `Cargo.toml` members list; crate
  `Cargo.toml` uses `version/edition/license/rust-version/repository
  .workspace = true`, `publish = false` unless the crate is intentionally
  part of the public AQC release surface,
  `[package.metadata.guardrail3] shared = true`, `[lints] workspace = true`.

## Where the rules live (when this file is doubted)

The compiled sources are in the guardrail3 repo (v1, frozen):
`packages/rs/code/g3rs-code-source-checks/` (fs bans, expect quality, import
caps), `packages/rs/arch/g3rs-arch-source-checks/` (facade rules, feature
contract), `apps/guardrail3-rs/.../topology.rs` + `marker_pairs.rs`
(placement, adoption markers, the `behavior/fixtures` walk exemption),
`g3rs-code-ingestion/.../classify.rs` (what counts as a test file). This
file mirrors them; if they disagree, the binary wins and this file needs the
amendment.
