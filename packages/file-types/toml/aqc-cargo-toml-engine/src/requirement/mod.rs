//! Declarative requirement and assertion types accepted by `CargoTomlEngine`.

#![expect(
    clippy::module_name_repetitions,
    reason = "Public re-exports preserve Cargo domain names for downstream callers."
)]

mod cargo_toml;
mod dependencies;
mod features;
mod lints;
mod package;
mod profiles;
mod sections;
mod targets;
mod workspace;

pub use cargo_toml::{
    CargoTomlRequirements, DependencyForbiddenGlobConflictBlocks, ResolvedCargoTomlRequirements,
};
pub use dependencies::DependencyIdentity;
pub use dependencies::DependencyKind;
pub use dependencies::DependencyPackageGlob;
pub use dependencies::DependencyRequirement;
pub use dependencies::DependencyScope;
pub use dependencies::DependencySpec;
pub use features::FeatureMembers;
pub use lints::{LintSetting, PackageLintsAssertion, ResolvedPackageLintsAssertion};
pub use package::{PackageFieldAssertion, ResolvedPackageFieldAssertion};
pub use profiles::{ProfileRequirements, ResolvedProfileRequirements};
pub use sections::ManifestSection;
pub use sections::SectionPresenceAssertion;
pub use targets::{
    ResolvedTargetFieldAssertion, ResolvedTargetRequirements, ResolvedTargetTableAssertion,
    TargetFieldAssertion, TargetRequirements, TargetTableAssertion,
};
pub use workspace::{ResolvedWorkspaceFieldAssertion, WorkspaceFieldAssertion};
