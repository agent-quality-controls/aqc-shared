//! Rust toolchain requirement model and merge logic.

mod merge;
mod model;

pub use model::{
    ResolvedRustToolchainTomlRequirements, RustToolchainChannel, RustToolchainPath,
    RustToolchainProfile, RustToolchainTomlRequirements, RustToolchainValueError,
};
