# Rust Syntax Crate

## Summary

Added `aqc-rust-syntax`, a file-local Rust syntax fact crate whose first public
surface is enum declarations and variants.

## Decisions Made

- Placed the crate under `packages/source/rust/aqc-rust-syntax` because it
  parses Rust source text, not a config file type.
- Kept the API file-local: callers pass one source string and receive enum
  facts or a parse error.
- Used `syn` with span locations and did not add filesystem, Cargo, crate
  walking, policy findings, Specular types, g3 types, or regex fallback.
- Collected only file-scope enums and enums inside inline `mod { ... }`
  blocks; `mod name;` declarations are intentionally not followed.

## Key Files For Context

- `packages/source/rust/aqc-rust-syntax/Cargo.toml`
- `packages/source/rust/aqc-rust-syntax/src/lib.rs`
- `packages/source/rust/aqc-rust-syntax/tests/enums.rs`
- `Cargo.toml`

## Verification

- `cargo test -p aqc-rust-syntax`
- `cargo clippy -p aqc-rust-syntax --all-targets`

## Next Steps

Publish `aqc-rust-syntax` when the Specular implementation is ready to depend
on a crates.io release instead of local source.
