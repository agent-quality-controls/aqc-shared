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
    LintLevelsAssertion, PackageFieldAssertion, ProfileAssertion, ProfileFieldAssertion,
};
