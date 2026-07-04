//! Rust toolchain requirement model and merge logic.

mod merge;
mod model;
mod settings;

pub use model::{
    ResolvedRustToolchainClosedSettings, ResolvedRustToolchainScalarSettings,
    ResolvedRustToolchainTomlRequirements, RustToolchainScalarSettings,
    RustToolchainTomlRequirements,
};
pub use settings::{RustToolchainListSetting, RustToolchainScalarSetting};
