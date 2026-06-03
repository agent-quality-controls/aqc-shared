//! `FileEngine` for `Cargo.toml`. Writes manifest tables via `toml_edit`.
//!
//! See plan refs in the architecture and vertical-slice plans inside
//! `guardrail3/.plans/g3v2-architecture/`.

#[cfg(feature = "api")]
mod engine;
#[cfg(feature = "api")]
mod reconcile;
#[cfg(feature = "api")]
mod requirement;

#[cfg(feature = "api")]
pub use engine::CargoTomlEngine;
#[cfg(feature = "api")]
pub use requirement::{
    CargoTomlRequirement, DepKind, DependencySetAssertion, DependencySpec, FeatureSetAssertion,
    LintLevelsAssertion, LintsInheritAssertion, PackageFieldAssertion, ProfileAssertion,
    ProfileFieldAssertion,
};

/// Stable engine id; matches this crate's `[package].name` and the value
/// returned by `<CargoTomlRequirement as EngineRequirement>::engine_id`.
#[cfg(feature = "api")]
pub const ENGINE_ID: &str = "aqc-cargo-toml-engine";
