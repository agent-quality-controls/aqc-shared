# aqc-clippy-toml-parser

Typed parser for `clippy.toml`, with the schema generated from upstream
[`rust-lang/rust-clippy`](https://github.com/rust-lang/rust-clippy) source
rather than hand-maintained.

This is a facade workspace with three member crates:

- `aqc-clippy-toml-parser` (this crate) - public facade. Re-exports the
  typed API from the runtime crate behind a `pub mod types`.
- `aqc-clippy-toml-parser-runtime` (`crates/runtime/`) - read/write API on
  top of the generated types. Currently a stub that re-exports types.
- `aqc-clippy-toml-parser-types` (`crates/types/`) - generated typed schema.
  Source-of-truth is `crates/types/src/clippy_toml.rs`, which is emitted by
  the generator binary. **Do not hand-edit.**
- `aqc-clippy-toml-parser-generator` (`crates/generator/`) - binary that
  downloads pinned upstream `conf.rs`, parses the `define_Conf!` macro, and
  rewrites `crates/types/src/clippy_toml.rs`.

## Regenerate

From the facade workspace root:

```sh
cargo run -p aqc-clippy-toml-parser-generator --bin generate
```

The pinned Clippy tag is the const `CLIPPY_TAG` in
`crates/generator/src/main.rs`, mirrored in `manifest.toml`. Bump both
together and rerun. The script caches the downloaded `conf.rs` at the
workspace root for offline reruns.

## Verify

```sh
./verify/verify-all.sh
```

Runs five mechanical layers (tree, public API, generator contract, generated
code contract, compilation) against the manifest. The verifier is the only
acceptance criterion for "the generator produced something valid."
