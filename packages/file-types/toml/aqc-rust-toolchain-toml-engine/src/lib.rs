//! `FileEngine` for `rust-toolchain.toml`.

#[cfg(feature = "api")]
mod engine;
#[cfg(feature = "api")]
mod reconcile;
#[cfg(feature = "api")]
mod requirement;

#[cfg(feature = "api")]
pub use aqc_file_engine_core::{ConfigScalar, ListRequirements, ScalarAssertion};
#[cfg(feature = "api")]
pub use engine::RustToolchainTomlEngine;
#[cfg(feature = "api")]
pub use requirement::{
    ResolvedRustToolchainClosedSettings, ResolvedRustToolchainScalarSettings,
    ResolvedRustToolchainTomlRequirements, RustToolchainListSetting, RustToolchainScalarSetting,
    RustToolchainScalarSettings, RustToolchainTomlRequirements,
};

#[cfg(feature = "api")]
pub const ENGINE_ID: &str = "aqc-rust-toolchain-toml-engine";
