# Rust Toolchain TOML Engine Spec Coverage

## Goal

- Covered by `tree.required`, dependency built-ins, custom `changed-file-scope`, and custom `symbol-scope`.

## Approved Flow

- Covered by dependency built-ins forbidding Shakrs, Cargo engine, old parser, and policy/adapter crates.
- Covered by content checks forbidding Shakrs, Cargo, old g3rs, and old parser imports.

## File Format Input

- Covered by setting enumerations.
- Covered by reconcile test content requirements for missing file, table, channel, components, targets, profile, path conflicts, list order, relative path, invalid profile, empty table, open unknown settings, and closed settings.

## Module 1: `aqc-rust-toolchain-toml-engine`

- Covered by `tree.required`.
- Covered by content checks for public facade, requirement model, settings, engine target path, and tests.
- Covered by custom `engine-contract`.
- Covered by custom `cargo-tests`.

## Module 2: `shakrs-toolchain-adapter`

- Not applicable in this repo.
- Covered by the Shakrs repo spec.

## Module 3: `shakrs-toolchain-policy`

- Not applicable in this repo.
- Covered by the Shakrs repo spec.

## App Integration

- Not applicable in this repo.
- Covered by the Shakrs repo spec.

## Config Integration

- Not applicable in this repo.
- Covered by the Shakrs repo spec.

## Fixture3 Coverage

- Not applicable in this repo.
- Covered by the Shakrs repo spec because fixtures exercise the full vertical through the CLI.

## Specular Coverage

- Covered by this spec file and the custom verifier under `specs/verifiers`.
- Covered by `specular lint`.

## Release Work

- Covered by dependency built-ins enough to catch path dependencies and forbidden package deps.
- Crates.io publication is release-time verification, not static repo-state verification.

## Decisions

- Covered by content checks and dependency checks forbidding old parser, old g3rs, Shakrs, Cargo, and extra nested requirement wrappers.

## UNCOVERED

- None.
