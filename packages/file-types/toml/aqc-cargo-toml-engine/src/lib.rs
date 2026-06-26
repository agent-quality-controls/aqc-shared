//! `FileEngine` for `Cargo.toml`. Writes manifest tables via `toml_edit`.
//!
//! See plan refs in the architecture and vertical-slice plans inside
//! `guardrail3/.plans/g3v2-architecture/`.

#[cfg(feature = "api")]
mod engine;
#[cfg(feature = "api")]
mod reconcile;
#[cfg(feature = "api")]
pub mod requirement;

#[cfg(feature = "api")]
pub use aqc_file_engine_core::{
    ConfigScalar, DottedVersion, ForbiddenGlobRequirements, ItemRequirements, KeyedItem,
    ScalarAssertion,
};
#[cfg(feature = "api")]
pub use engine::CargoTomlEngine;
#[cfg(feature = "api")]
pub type CargoTomlRequirements = requirement::CargoTomlRequirements;
#[cfg(feature = "api")]
pub use requirement::cargo_lint_table_requirements;
#[cfg(feature = "api")]
pub type DependencyForbiddenGlobConflictBlocks = requirement::DependencyForbiddenGlobConflictBlocks;
#[cfg(feature = "api")]
pub type DependencyIdentity = requirement::DependencyIdentity;
#[cfg(feature = "api")]
pub type DependencyKind = requirement::DependencyKind;
#[cfg(feature = "api")]
pub type DependencyPackageGlob = requirement::DependencyPackageGlob;
#[cfg(feature = "api")]
pub type DependencyRequirement = requirement::DependencyRequirement;
#[cfg(feature = "api")]
pub type DependencyScope = requirement::DependencyScope;
#[cfg(feature = "api")]
pub type DependencySpec = requirement::DependencySpec;
#[cfg(feature = "api")]
pub type FeatureMembers = requirement::FeatureMembers;
#[cfg(feature = "api")]
pub type LintSetting = requirement::LintSetting;
#[cfg(feature = "api")]
pub type ManifestSection = requirement::ManifestSection;
#[cfg(feature = "api")]
pub type PackageFieldAssertion = requirement::PackageFieldAssertion;
#[cfg(feature = "api")]
pub type PackageLintsAssertion = requirement::PackageLintsAssertion;
#[cfg(feature = "api")]
pub type ProfileRequirements = requirement::ProfileRequirements;
#[cfg(feature = "api")]
pub type ResolvedCargoTomlRequirements = requirement::ResolvedCargoTomlRequirements;
#[cfg(feature = "api")]
pub type ResolvedPackageFieldAssertion = requirement::ResolvedPackageFieldAssertion;
#[cfg(feature = "api")]
pub type ResolvedTargetFieldAssertion = requirement::ResolvedTargetFieldAssertion;
#[cfg(feature = "api")]
pub type ResolvedTargetTableAssertion = requirement::ResolvedTargetTableAssertion;
#[cfg(feature = "api")]
pub type ResolvedWorkspaceFieldAssertion = requirement::ResolvedWorkspaceFieldAssertion;
#[cfg(feature = "api")]
pub type SectionPresenceAssertion = requirement::SectionPresenceAssertion;
#[cfg(feature = "api")]
pub type TargetFieldAssertion = requirement::TargetFieldAssertion;
#[cfg(feature = "api")]
pub type TargetRequirements = requirement::TargetRequirements;
#[cfg(feature = "api")]
pub type TargetTableAssertion = requirement::TargetTableAssertion;
#[cfg(feature = "api")]
pub type WorkspaceFieldAssertion = requirement::WorkspaceFieldAssertion;

/// Stable engine id; matches this crate's `[package].name` and the value
/// returned by `<CargoTomlRequirements as EngineRequirement>::engine_id`.
#[cfg(feature = "api")]
pub const ENGINE_ID: &str = "aqc-cargo-toml-engine";
