//! Rust toolchain requirement model and merge logic.

#![allow(
    clippy::missing_docs_in_private_items,
    clippy::missing_errors_doc,
    clippy::missing_const_for_fn,
    clippy::ref_option,
    reason = "Rust toolchain requirement internals directly mirror rust-toolchain.toml fields; public types are the documentation boundary."
)]

mod merge;
mod model;

pub use model::{
    ResolvedRustToolchainTomlRequirements, RustToolchainChannel, RustToolchainPath,
    RustToolchainProfile, RustToolchainTomlRequirements, RustToolchainValueError,
};
