//! File-local Rust syntax facts.
//!
//! This crate parses one Rust source string and returns syntax facts. It does
//! not read files, walk crates, resolve modules, or attach policy findings.

#[cfg(feature = "api")]
mod model;
#[cfg(feature = "api")]
mod parser;

#[cfg(feature = "api")]
pub use model::{RustEnumDecl, RustFileSyntax, RustSyntaxError, RustVisibility};
#[cfg(feature = "api")]
pub use parser::parse_rust_syntax;
