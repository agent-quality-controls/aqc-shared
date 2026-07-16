//! `FileEngine` for `deny.toml`.

#[cfg(feature = "api")]
mod engine;
#[cfg(feature = "api")]
mod reconcile;
#[cfg(feature = "api")]
pub mod requirement;

#[cfg(feature = "api")]
pub use aqc_file_engine_core::{
    ConflictEntry, ItemRequirements, KeyedItem, ListRequirements, Provenance,
    ResolvedItemRequirements, ResolvedListRequirements, ResolvedRequirement, ScalarAssertion,
    ScalarValue,
};
#[cfg(feature = "api")]
pub use engine::DenyTomlEngine;
#[cfg(feature = "api")]
pub type DenyAdvisoryIgnoreIdentity = requirement::DenyAdvisoryIgnoreIdentity;
#[cfg(feature = "api")]
pub type DenyAdvisoryIgnoreSpec = requirement::DenyAdvisoryIgnoreSpec;
#[cfg(feature = "api")]
pub type DenyAdvisoryScope = requirement::DenyAdvisoryScope;
#[cfg(feature = "api")]
pub type DenyBanSpec = requirement::DenyBanSpec;
#[cfg(feature = "api")]
pub type DenyBuildGlobSpec = requirement::DenyBuildGlobSpec;
#[cfg(feature = "api")]
pub type DenyConfidenceThreshold = requirement::DenyConfidenceThreshold;
#[cfg(feature = "api")]
pub type DenyDuration = requirement::DenyDuration;
#[cfg(feature = "api")]
pub type DenyFeatureBanSpec = requirement::DenyFeatureBanSpec;
#[cfg(feature = "api")]
pub type DenyGitSpec = requirement::DenyGitSpec;
#[cfg(feature = "api")]
pub type DenyGraphHighlight = requirement::DenyGraphHighlight;
#[cfg(feature = "api")]
pub type DenyGraphTargetSpec = requirement::DenyGraphTargetSpec;
#[cfg(feature = "api")]
pub type DenyLicenseClarification = requirement::DenyLicenseClarification;
#[cfg(feature = "api")]
pub type DenyLicenseException = requirement::DenyLicenseException;
#[cfg(feature = "api")]
pub type DenyLicenseFile = requirement::DenyLicenseFile;
#[cfg(feature = "api")]
pub type DenyLintLevel = requirement::DenyLintLevel;
#[cfg(feature = "api")]
pub type DenyNonEmptyString = requirement::DenyNonEmptyString;
#[cfg(feature = "api")]
pub type DenyPackageReasonSpec = requirement::DenyPackageReasonSpec;
#[cfg(feature = "api")]
pub type DenyPackageSpec = requirement::DenyPackageSpec;
#[cfg(feature = "api")]
pub type DenySkipTreeSpec = requirement::DenySkipTreeSpec;
#[cfg(feature = "api")]
pub type DenyTable = requirement::DenyTable;
#[cfg(feature = "api")]
pub type DenyTomlRequirements = requirement::DenyTomlRequirements;
#[cfg(feature = "api")]
pub type DenyTomlValueError = requirement::DenyTomlValueError;
#[cfg(feature = "api")]
pub type ResolvedDenyTomlRequirements = requirement::ResolvedDenyTomlRequirements;

#[cfg(feature = "api")]
pub const ENGINE_ID: &str = "aqc-deny-toml-engine";
