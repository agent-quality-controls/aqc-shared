# Generator pipes output through rustfmt

## Summary

Made `aqc-clippy-toml-parser-generator` pipe its emitted Rust through
`rustfmt --edition 2024 --emit stdout` before writing to disk. Closes the
gap where regenerating the schema would dirty `cargo fmt --check` until a
follow-up `cargo fmt` ran.

## Decisions made

- **Format-on-emit, not format-on-commit.** The generator owns the
  formatting contract. Anyone who runs the generator and stages the result
  gets bit-identical output, regardless of what their local `cargo fmt`
  would do.
- **Subprocess + stdin/stdout pipe**, not a rustfmt library dep. Avoids
  pinning a rustfmt crate version that drifts from the toolchain.
- **Subprocess code lives in `fs.rs`** alongside the `std::fs` access. The
  `fs.rs` module is the single I/O boundary for the generator binary; one
  place to grep for "side effects."
- **Hard fail if `rustfmt` is missing.** It ships with every Rust
  toolchain we'd run this on; a missing rustfmt means the toolchain is
  broken and we want to know.

## Verification

- `cargo run -p aqc-clippy-toml-parser-generator --bin generate` →
  emits formatted output, "rustfmt applied" line printed.
- `cargo fmt --all -- --check`: clean after regeneration.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`:
  clean.
- `./verify/verify-all.sh`: all 5 layers green.

## Key files for context

- `crates/generator/src/fs.rs` - new `write_formatted_rust` and private
  `rustfmt()` helper.
- `crates/generator/src/main.rs` - swapped `fs::write_file` for
  `fs::write_formatted_rust`.

## Next steps

Same as the previous worklog -- engine layer, cargo parser, vertical
slice cutover. No new work surfaced by this change.
