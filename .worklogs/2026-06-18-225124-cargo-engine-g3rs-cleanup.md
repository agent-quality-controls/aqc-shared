Summary:
- Made the remaining cargo TOML engine clippy, duplication, and full g3rs workspace gates pass.
- Kept helper extraction inside `aqc-cargo-toml-engine` instead of changing `aqc-file-engine-core`, so publish dry-runs still verify against the already-published core API.

Decisions made:
- Added local, reasoned clippy expectations for cargo requirement/reconcile modules where lints conflict with Cargo requirement data shapes and provenance tuple composition.
- Fixed direct issues where the code was just noisy or brittle: shadowed names, redundant closures, `&Option<T>` parameters, test casts, test indexing, and duplicate helper code.
- Moved shared cargo requirement helpers into `requirement/helpers.rs` because `requirement/mod.rs` must remain facade-only under g3rs.
- Did not bump workspace versions because the temporary core API extraction made dependent package dry-runs require an unpublished `aqc-file-engine-core` version.

Verification:
- `cargo clippy -p aqc-file-engine-core -p aqc-cargo-toml-engine -p aqc-clippy-toml-engine --all-targets --all-features -- -D warnings`
- `cargo test -p aqc-file-engine-core -p aqc-cargo-toml-engine -p aqc-clippy-toml-engine`
- `cargo dupes check --min-lines 8 --max-exact 85 --max-exact-percent 10`
- `cargo dupes check --min-lines 8 --max-exact 85 --max-exact-percent 10 --exclude-tests`
- `cargo publish --dry-run --allow-dirty -p aqc-cargo-toml-engine`
- `cargo publish --dry-run --allow-dirty -p aqc-clippy-toml-engine`
- `g3rs validate workspace --path .`

Key files for context:
- `packages/file-types/toml/aqc-cargo-toml-engine/src/requirement/helpers.rs`
- `packages/file-types/toml/aqc-cargo-toml-engine/src/requirement/*.rs`
- `packages/file-types/toml/aqc-cargo-toml-engine/src/reconcile/**/*.rs`
- `packages/file-types/toml/aqc-cargo-toml-engine/tests/*.rs`
- `packages/file-types/toml/aqc-clippy-toml-engine/src/requirement/scalar.rs`

Next steps:
- The old g3rs error-level blockers are cleared. Remaining g3rs findings are warnings and existing waivers.
