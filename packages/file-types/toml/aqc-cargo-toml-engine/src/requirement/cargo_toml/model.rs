//! Public `Cargo.toml` requirement model types.

#![expect(
    clippy::disallowed_types,
    reason = "`Any` is used only for EngineRequirement downcast dispatch."
)]

use core::any::Any;
use std::collections::{BTreeMap, BTreeSet};

use aqc_file_engine_core as core_types;

use super::super::{
    dependencies as deps, features, lints, package, profiles, sections, targets, workspace,
};

#[derive(Debug, Clone, Default)]
pub struct CargoTomlRequirements {
    pub package_lints: Option<lints::PackageLintsAssertion>,
    pub workspace_lints:
        BTreeMap<String, core_types::ItemRequirements<core_types::KeyedItem<lints::LintSetting>>>,
    pub package_fields: BTreeMap<String, package::PackageFieldAssertion>,
    pub workspace_package_fields: BTreeMap<String, package::PackageFieldAssertion>,
    pub workspace_fields: BTreeMap<String, workspace::WorkspaceFieldAssertion>,
    pub section_presence: BTreeMap<sections::ManifestSection, sections::SectionPresenceAssertion>,
    pub dependencies:
        BTreeMap<deps::DependencyScope, core_types::ItemRequirements<deps::DependencyRequirement>>,
    pub forbidden_dependency_package_globs: BTreeMap<
        deps::DependencyScope,
        core_types::ForbiddenGlobRequirements<deps::DependencyPackageGlob>,
    >,
    pub workspace_dependencies: Option<core_types::ItemRequirements<deps::DependencyRequirement>>,
    pub forbidden_workspace_dependency_package_globs:
        Option<core_types::ForbiddenGlobRequirements<deps::DependencyPackageGlob>>,
    pub features:
        Option<core_types::ItemRequirements<core_types::KeyedItem<features::FeatureMembers>>>,
    pub profiles: BTreeMap<String, profiles::ProfileRequirements>,
    pub targets: targets::TargetRequirements,
    pub patch: BTreeMap<String, core_types::ItemRequirements<deps::DependencyRequirement>>,
    pub forbidden_patch_dependency_package_globs:
        BTreeMap<String, core_types::ForbiddenGlobRequirements<deps::DependencyPackageGlob>>,
}

#[rustfmt::skip]
#[derive(Debug, Clone, Default)]
pub struct ResolvedCargoTomlRequirements {
    pub package_lints: Option<lints::ResolvedPackageLintsAssertion>,
    pub workspace_lints: BTreeMap<String, core_types::ResolvedItemRequirements<core_types::KeyedItem<lints::LintSetting>>>,
    pub package_fields:
        BTreeMap<String, core_types::ResolvedRequirement<package::ResolvedPackageFieldAssertion, package::PackageFieldAssertion>>,
    pub workspace_package_fields:
        BTreeMap<String, core_types::ResolvedRequirement<package::ResolvedPackageFieldAssertion, package::PackageFieldAssertion>>,
    pub workspace_fields:
        BTreeMap<String, core_types::ResolvedRequirement<workspace::ResolvedWorkspaceFieldAssertion, workspace::WorkspaceFieldAssertion>>,
    pub section_presence: BTreeMap<sections::ManifestSection, core_types::ResolvedRequirement<sections::SectionPresenceAssertion, sections::SectionPresenceAssertion>>,
    pub dependencies: BTreeMap<deps::DependencyScope, core_types::ResolvedItemRequirements<deps::DependencyRequirement>>,
    pub forbidden_dependency_package_globs:
        BTreeMap<deps::DependencyScope, core_types::ResolvedForbiddenGlobRequirements<deps::DependencyPackageGlob>>,
    pub dependency_glob_conflicts:
        BTreeMap<deps::DependencyScope, DependencyForbiddenGlobConflictBlocks>,
    pub workspace_dependencies: Option<core_types::ResolvedItemRequirements<deps::DependencyRequirement>>,
    pub forbidden_workspace_dependency_package_globs:
        Option<core_types::ResolvedForbiddenGlobRequirements<deps::DependencyPackageGlob>>,
    pub workspace_dependency_glob_conflicts: DependencyForbiddenGlobConflictBlocks,
    pub features: Option<core_types::ResolvedItemRequirements<core_types::KeyedItem<features::FeatureMembers>>>,
    pub profiles: BTreeMap<String, profiles::ResolvedProfileRequirements>,
    pub targets: targets::ResolvedTargetRequirements,
    pub patch: BTreeMap<String, core_types::ResolvedItemRequirements<deps::DependencyRequirement>>,
    pub forbidden_patch_dependency_package_globs:
        BTreeMap<String, core_types::ResolvedForbiddenGlobRequirements<deps::DependencyPackageGlob>>,
    pub patch_dependency_glob_conflicts:
        BTreeMap<String, DependencyForbiddenGlobConflictBlocks>,
}

#[derive(Debug, Clone, Default)]
pub struct DependencyForbiddenGlobConflictBlocks {
    pub required: BTreeSet<deps::DependencyIdentity>,
    pub package_globs: BTreeSet<String>,
}

impl DependencyForbiddenGlobConflictBlocks {
    pub(super) fn is_empty(&self) -> bool {
        self.required.is_empty() && self.package_globs.is_empty()
    }
}

impl core_types::EngineRequirement for CargoTomlRequirements {
    fn engine_id(&self) -> &'static str {
        crate::ENGINE_ID
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
