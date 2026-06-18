//! `FileEngine` for `rustfmt.toml`. Writes Rustfmt scalar and list settings
//! via `toml_edit`.

#[cfg(feature = "api")]
mod engine;
#[cfg(feature = "api")]
mod reconcile;
#[cfg(feature = "api")]
mod requirement;

#[cfg(feature = "api")]
pub use engine::RustfmtTomlEngine;
#[cfg(feature = "api")]
pub use requirement::{
    ResolvedRustfmtScalarAssertion, ResolvedRustfmtTomlRequirements, RustfmtListSetting,
    RustfmtScalarAssertion, RustfmtScalarSetting, RustfmtTomlRequirements,
};

/// Stable engine id; matches this crate's `[package].name` and the value
/// returned by `<RustfmtTomlRequirements as EngineRequirement>::engine_id`.
#[cfg(feature = "api")]
pub const ENGINE_ID: &str = "aqc-rustfmt-toml-engine";
