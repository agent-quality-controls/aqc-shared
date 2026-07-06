//! `FileEngine` for `rust-toolchain.toml`.

#[cfg(feature = "api")]
mod engine;
#[cfg(feature = "api")]
mod reconcile;
#[cfg(feature = "api")]
mod requirement;

#[cfg(feature = "api")]
pub use aqc_file_engine_core::{
    ConflictEntry, ListRequirements, Provenance, ResolvedListRequirements, ResolvedRequirement,
    ScalarAssertion, ScalarValue,
};
#[cfg(feature = "api")]
pub use engine::RustToolchainTomlEngine;
#[cfg(feature = "api")]
pub use requirement::{
    ResolvedRustToolchainTomlRequirements, RustToolchainChannel, RustToolchainPath,
    RustToolchainProfile, RustToolchainTomlRequirements, RustToolchainValueError,
};

#[cfg(feature = "api")]
pub const ENGINE_ID: &str = "aqc-rust-toolchain-toml-engine";
