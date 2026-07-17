# Membership Checker And Local Gates

## Summary

Hardened the permanent requirement-architecture checker, completed explicit membership support, and replaced architecture CI with cached local gates. Every AQC production workspace and Fixture3 suite is now checked from stable clone-local Cargo state.

## Decisions Made

- Canonical core vocabulary must resolve to the configured `aqc-file-engine-core` manifest or the default crates.io registry; matching package names from paths, Git, or alternate registries are not trusted.
- Local bindings lose tracked membership provenance when any Rust binding form shadows them.
- Generated Cargo patch configuration is replaced atomically, and fixture targets live under the clone's `.cargo-target/fixtures/` directory.
- Unchanged generated Cargo configuration is left in place so repeated local runs preserve Cargo fingerprints instead of recompiling every dependency.
- Specular and the workspace gate use the same generated Cargo home and target identity. They no longer alternate inline and file-based patch configuration that invalidated each other's fingerprints.
- The live architecture scan uses the same generated Cargo environment as the checker's tests, including detached pre-push execution.
- The pre-commit entry point uses the modular dispatcher syntax required by G3RS and sources both registered fragments without embedding either tool's commands in the dispatcher.
- Each pre-commit workspace check receives only its transitive local package patches, so unpublished current-version dependencies resolve without cross-workspace path dependencies or unused-patch lock changes.
- One repository-local command wrapper supplies that Cargo environment to G3RS and Shakrs without changing either generated fragment or hiding the dispatcher contract from source validation.
- Architecture-checker fixture workspaces run static G3RS rules only; the parent checker workspace test gate owns their compilation and behavior, avoiding duplicate MSRV builds of deliberately minimal fixtures.
- Local Cargo patches are generated only for registry dependencies; explicit path dependencies keep their own lock-file identity.
- Staged G3RS checks materialize one content-addressed index snapshot and derive direct and transitive local patches from it, so package metadata, source bytes, and dependency selection cannot mix staged and unstaged state.
- Manifest discovery excludes generated/support roots precisely; production crates cannot hide below generic directory names such as `tests` or `specs`.
- Executable local-source tests cover concurrent atomic writes, unchanged metadata, explicit path dependencies, and staged-index selection.
- Deny's exhaustive schema-shaped merge mapping uses the standardized effective-line waiver; splitting it would hide completeness across partial field inventories.
- Pre-push accepts only checked-out `HEAD` from a clean repository; the Shakrs pre-commit fragment validates staged bytes.
- Pre-push runs the gate from a detached worktree at pushed `HEAD`; repository-relative identities reuse the repository's Cargo and Fixture3 caches.
- The checker rejects helper-parameter laundering, typed and inferred closure laundering, policy-membership discard, same-name destructuring, helper-returned whole requirements through every modeled destructuring form, and counterfeit nested facade paths. Required test names must appear in Cargo's executable test inventory.
- Canonical public re-exports are keyed by manifest identity, not package name. The checker follows Cargo target source paths, external modules, and `#[path]` modules instead of scanning an assumed `src` directory.
- Helper-returned engine requirements, inherent adapter `self`, parameterized macros, qualified same-name impostors, scoped trait aliases, custom Cargo targets, and external path modules have dedicated adversarial fixtures.
- Requirement root declarations are paired to structs by source and module identity. Core vocabulary import renames are rejected; semantic aliases for distinct composed types remain supported.
- Semantic type and requirement-trait aliases resolve in their declaring modules, so sibling modules cannot overwrite each other's identities. Qualified local adapter roots remain tracked while qualified same-name impostors remain unrelated.
- Scoped import resolution follows renamed and chained parent imports, honors local same-name shadows, and discovers path modules nested in inline modules. Closure tracking follows direct, aliased, destructured, typed, and reassigned bindings.
- Requirement crates reject glob and block-local imports as opaque surfaces. Module imports of semantic type aliases remain supported and inventoried; generic parameters and unrelated same-terminal output types retain their own identities.
- Requirement traits imported from canonical provider crates are resolved through direct and multi-hop public re-exports to a fixed point. The permanent verifier compares the production inventory against the complete expected root set, so an empty or partial scan fails.
- Parallel gate workers explicitly propagate each command status, so metadata, cargo-deny, or Clippy failures cannot be hidden by a later successful test command.
- A fast executable failure-injection check proves each workspace stage stops at metadata, cargo-deny, Clippy, or test failure even though the logged worker invokes reconciliation inside a shell conditional.
- Once strict-Clippy failures could no longer be hidden, JSON switched to lazy fallback evaluation and YAML avoided a redundant token-conversion closure without changing parse results.
- Every production tool workspace is discovered automatically; tools without a local Clippy file use the repository Clippy configuration.
- Gate logs are isolated by run, while Cargo homes, registry data, and compilation targets remain stable across runs. The complete warm local gate passes in about 40 seconds on the current machine.
- The release workflow remains release orchestration and does not replace the local development gate.

## Key Files For Context

- `.plans/2026-07-16-144118-explicit-setting-membership.md`
- `.plans/2026-07-16-205526-fast-local-gates.md`
- `tools/aqc-requirement-architecture/src/analyze.rs`
- `tools/aqc-requirement-architecture/src/expression.rs`
- `specs/explicit-setting-membership.spec.json`
- `scripts/check-workspaces.sh`
- `scripts/local_cargo_source.py`

## Next Steps

- Keep the permanent Specular contract and all local workspace and fixture gates passing as core vocabulary expands.
