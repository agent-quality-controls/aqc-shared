//! Public `Cargo.toml` requirement model types.

#![allow(
    clippy::disallowed_types,
    reason = "`Any` is used only for EngineRequirement downcast dispatch."
)]
#![cfg_attr(
    not(test),
    expect(
        clippy::missing_docs_in_private_items,
        reason = "Private aggregate model helpers support the shared requirement API."
    )
)]
#![allow(
    clippy::type_complexity,
    reason = "Cargo aggregate structs expose field-per-section requirement shapes."
)]

use core::any::Any;
use std::collections::BTreeMap;

use aqc_file_engine_core as core_types;

use super::super::{
    dependencies as deps, features, lints, package, profiles, sections, targets, workspace,
};

#[derive(Debug, Clone, Default)]
pub struct CargoTomlRequirements {
    pub package_lints: Option<lints::PackageLintsAssertion>,
    pub package_lint_tables: core_types::ItemRequirements<core_types::KeyedItem<()>>,
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
    pub(crate) package_lints: Option<lints::ResolvedPackageLintsAssertion>,
    pub(crate) package_lint_tables: core_types::ResolvedItemRequirements<core_types::KeyedItem<()>>,
    pub(crate) workspace_lints: BTreeMap<String, core_types::ResolvedItemRequirements<core_types::KeyedItem<lints::LintSetting>>>,
    pub(crate) package_fields:
        BTreeMap<String, core_types::ResolvedRequirement<package::ResolvedPackageFieldAssertion, package::PackageFieldAssertion>>,
    pub(crate) workspace_package_fields:
        BTreeMap<String, core_types::ResolvedRequirement<package::ResolvedPackageFieldAssertion, package::PackageFieldAssertion>>,
    pub(crate) workspace_fields:
        BTreeMap<String, core_types::ResolvedRequirement<workspace::ResolvedWorkspaceFieldAssertion, workspace::WorkspaceFieldAssertion>>,
    pub(crate) section_presence: BTreeMap<sections::ManifestSection, core_types::ResolvedRequirement<sections::SectionPresenceAssertion, sections::SectionPresenceAssertion>>,
    pub(crate) dependencies: BTreeMap<deps::DependencyScope, core_types::ResolvedItemRequirements<deps::DependencyRequirement>>,
    pub(crate) forbidden_dependency_package_globs:
        BTreeMap<deps::DependencyScope, core_types::ResolvedForbiddenGlobRequirements<deps::DependencyPackageGlob>>,
    pub(crate) workspace_dependencies: Option<core_types::ResolvedItemRequirements<deps::DependencyRequirement>>,
    pub(crate) forbidden_workspace_dependency_package_globs:
        Option<core_types::ResolvedForbiddenGlobRequirements<deps::DependencyPackageGlob>>,
    pub(crate) features: Option<core_types::ResolvedItemRequirements<core_types::KeyedItem<features::FeatureMembers>>>,
    pub(crate) profiles: BTreeMap<String, profiles::ResolvedProfileRequirements>,
    pub(crate) targets: targets::ResolvedTargetRequirements,
    pub(crate) patch: BTreeMap<String, core_types::ResolvedItemRequirements<deps::DependencyRequirement>>,
    pub(crate) forbidden_patch_dependency_package_globs:
        BTreeMap<String, core_types::ResolvedForbiddenGlobRequirements<deps::DependencyPackageGlob>>,
}

impl ResolvedCargoTomlRequirements {
    #[must_use]
    pub const fn package_lints(&self) -> Option<&lints::ResolvedPackageLintsAssertion> {
        self.package_lints.as_ref()
    }

    #[must_use]
    pub const fn package_lint_tables(
        &self,
    ) -> &core_types::ResolvedItemRequirements<core_types::KeyedItem<()>> {
        &self.package_lint_tables
    }

    #[must_use]
    pub const fn workspace_lints(
        &self,
    ) -> &BTreeMap<
        String,
        core_types::ResolvedItemRequirements<core_types::KeyedItem<lints::LintSetting>>,
    > {
        &self.workspace_lints
    }

    #[must_use]
    pub const fn package_fields(
        &self,
    ) -> &BTreeMap<
        String,
        core_types::ResolvedRequirement<
            package::ResolvedPackageFieldAssertion,
            package::PackageFieldAssertion,
        >,
    > {
        &self.package_fields
    }

    #[must_use]
    pub const fn workspace_package_fields(
        &self,
    ) -> &BTreeMap<
        String,
        core_types::ResolvedRequirement<
            package::ResolvedPackageFieldAssertion,
            package::PackageFieldAssertion,
        >,
    > {
        &self.workspace_package_fields
    }

    #[must_use]
    pub const fn workspace_fields(
        &self,
    ) -> &BTreeMap<
        String,
        core_types::ResolvedRequirement<
            workspace::ResolvedWorkspaceFieldAssertion,
            workspace::WorkspaceFieldAssertion,
        >,
    > {
        &self.workspace_fields
    }

    #[must_use]
    pub const fn section_presence(
        &self,
    ) -> &BTreeMap<
        sections::ManifestSection,
        core_types::ResolvedRequirement<
            sections::SectionPresenceAssertion,
            sections::SectionPresenceAssertion,
        >,
    > {
        &self.section_presence
    }

    #[must_use]
    pub const fn dependencies(
        &self,
    ) -> &BTreeMap<
        deps::DependencyScope,
        core_types::ResolvedItemRequirements<deps::DependencyRequirement>,
    > {
        &self.dependencies
    }

    #[must_use]
    pub const fn forbidden_dependency_package_globs(
        &self,
    ) -> &BTreeMap<
        deps::DependencyScope,
        core_types::ResolvedForbiddenGlobRequirements<deps::DependencyPackageGlob>,
    > {
        &self.forbidden_dependency_package_globs
    }

    #[must_use]
    pub const fn workspace_dependencies(
        &self,
    ) -> Option<&core_types::ResolvedItemRequirements<deps::DependencyRequirement>> {
        self.workspace_dependencies.as_ref()
    }

    #[must_use]
    pub const fn forbidden_workspace_dependency_package_globs(
        &self,
    ) -> Option<&core_types::ResolvedForbiddenGlobRequirements<deps::DependencyPackageGlob>> {
        self.forbidden_workspace_dependency_package_globs.as_ref()
    }

    #[must_use]
    pub const fn features(
        &self,
    ) -> Option<
        &core_types::ResolvedItemRequirements<core_types::KeyedItem<features::FeatureMembers>>,
    > {
        self.features.as_ref()
    }

    #[must_use]
    pub const fn profiles(&self) -> &BTreeMap<String, profiles::ResolvedProfileRequirements> {
        &self.profiles
    }

    #[must_use]
    pub const fn targets(&self) -> &targets::ResolvedTargetRequirements {
        &self.targets
    }

    #[must_use]
    pub const fn patch(
        &self,
    ) -> &BTreeMap<String, core_types::ResolvedItemRequirements<deps::DependencyRequirement>> {
        &self.patch
    }

    #[must_use]
    pub const fn forbidden_patch_dependency_package_globs(
        &self,
    ) -> &BTreeMap<String, core_types::ResolvedForbiddenGlobRequirements<deps::DependencyPackageGlob>>
    {
        &self.forbidden_patch_dependency_package_globs
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
