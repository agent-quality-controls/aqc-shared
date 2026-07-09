//! Deny TOML requirement model and merge logic.

#![allow(
    clippy::missing_docs_in_private_items,
    reason = "Private merge aliases and helpers are direct file-format wiring; the public requirement surface carries the useful documentation boundary."
)]

mod merge;
mod merge_helpers;
mod model;
mod value;

pub use model::{DenyTomlRequirements, ResolvedDenyTomlRequirements};
pub use value::{
    DenyAdvisoryIgnoreIdentity, DenyAdvisoryIgnoreSpec, DenyAdvisoryScope, DenyBanSpec,
    DenyBuildGlobSpec, DenyConfidenceThreshold, DenyDuration, DenyFeatureBanSpec, DenyGitSpec,
    DenyGraphHighlight, DenyGraphTargetSpec, DenyLicenseClarification, DenyLicenseException,
    DenyLicenseFile, DenyLintLevel, DenyNonEmptyString, DenyPackageReasonSpec, DenyPackageSpec,
    DenySkipTreeSpec, DenyTomlValueError,
};
