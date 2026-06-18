Summary:
- Made `aqc-clippy-toml-engine` pass its package clippy and test gates.
- Fixed concrete shadowing, nesting, bool simplification, needless return, cast, indexing, and panic findings; kept localized expectations for API and helper shapes where direct fixes would add noise.

Decisions made:
- Used `cfg_attr(not(test), expect(...))` for `missing_docs_in_private_items` in helper-heavy modules because clippy runs the library in test mode where that lint does not fire, making a plain `expect` fail.
- Kept type-complexity expectations on repeated resolved requirement shapes that mirror the public requirement model.
- Replaced test indexing and panic assertions with `get(...).expect(...)` plus `matches!`, preserving assertion intent without denied panic/indexing lints.

Verification:
- `cargo fmt -p aqc-clippy-toml-engine`
- `cargo clippy -p aqc-clippy-toml-engine --all-targets --all-features -- -D warnings`
- `cargo test -p aqc-clippy-toml-engine`

Key files for context:
- `packages/file-types/toml/aqc-clippy-toml-engine/src/requirement/model.rs`
- `packages/file-types/toml/aqc-clippy-toml-engine/src/requirement/scalar.rs`
- `packages/file-types/toml/aqc-clippy-toml-engine/src/requirement/merge.rs`
- `packages/file-types/toml/aqc-clippy-toml-engine/src/reconcile/*.rs`
- `packages/file-types/toml/aqc-clippy-toml-engine/tests/merge.rs`

Next steps:
- Run the cargo TOML engine clippy gate and fix its remaining findings as the next isolated batch.
