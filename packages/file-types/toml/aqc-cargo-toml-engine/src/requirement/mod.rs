//! Declarative requirement and assertion types accepted by `CargoTomlEngine`.

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
    CargoTomlRequirements, DependencyPatternConflictBlocks, ResolvedCargoTomlRequirements,
};
pub use dependencies::DependencyIdentity;
pub use dependencies::DependencyKind;
pub use dependencies::DependencyPackagePattern;
pub use dependencies::DependencyRequirement;
pub use dependencies::DependencyScope;
pub use dependencies::DependencySpec;
pub use features::FeatureMembers;
pub use lints::{LintSetting, PackageLintsAssertion, ResolvedPackageLintsAssertion};
pub use package::{PackageFieldAssertion, ResolvedPackageFieldAssertion};
pub use profiles::{ProfileFieldAssertion, ProfileRequirements, ResolvedProfileRequirements};
pub use sections::ManifestSection;
pub use sections::SectionPresenceAssertion;
pub use targets::{
    ResolvedTargetFieldAssertion, ResolvedTargetRequirements, ResolvedTargetTableAssertion,
    TargetFieldAssertion, TargetRequirements, TargetTableAssertion,
};
pub use workspace::{ResolvedWorkspaceFieldAssertion, WorkspaceFieldAssertion};
