//! Deny TOML requirement model types.

#![allow(
    clippy::disallowed_types,
    reason = "`Any` is used only for EngineRequirement downcast dispatch."
)]

use core::any::Any;
use std::collections::BTreeMap;

use aqc_file_engine_core::{
    EngineRequirement, ItemRequirements, KeyedItem, ListRequirements, ResolvedItemRequirements,
    ResolvedListRequirements, ResolvedRequirement, ScalarAssertion,
};

use super::value;

type ResolvedScalar<T> = Option<ResolvedRequirement<ScalarAssertion<T>, ScalarAssertion<T>>>;
type ResolvedScalarRef<'a, T> =
    Option<&'a ResolvedRequirement<ScalarAssertion<T>, ScalarAssertion<T>>>;

/// A modeled table whose direct keys can have explicit membership requirements.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DenyTable {
    Root,
    Graph,
    Output,
    Advisories,
    Licenses,
    LicensesPrivate,
    Bans,
    BansWorkspaceDependencies,
    BansBuild,
    Sources,
    SourcesAllowOrg,
}

#[derive(Debug, Clone, Default)]
pub struct DenyTomlRequirements {
    pub graph_targets: ItemRequirements<value::DenyGraphTargetSpec>,
    pub graph_exclude: ListRequirements,
    pub graph_exclude_dev: Option<ScalarAssertion<bool>>,
    pub graph_exclude_unpublished: Option<ScalarAssertion<bool>>,
    pub graph_all_features: Option<ScalarAssertion<bool>>,
    pub graph_no_default_features: Option<ScalarAssertion<bool>>,
    pub graph_features: ListRequirements,
    pub output_feature_depth: Option<ScalarAssertion<u64>>,
    pub advisories_version: Option<ScalarAssertion<u64>>,
    pub advisories_db_path: Option<ScalarAssertion<value::DenyNonEmptyString>>,
    pub advisories_db_urls: ListRequirements,
    pub advisories_yanked: Option<ScalarAssertion<value::DenyLintLevel>>,
    pub advisories_disable_yank_checking: Option<ScalarAssertion<bool>>,
    pub advisories_ignore: ItemRequirements<value::DenyAdvisoryIgnoreSpec>,
    pub advisories_unmaintained: Option<ScalarAssertion<value::DenyAdvisoryScope>>,
    pub advisories_unsound: Option<ScalarAssertion<value::DenyAdvisoryScope>>,
    pub advisories_maximum_db_staleness: Option<ScalarAssertion<value::DenyDuration>>,
    pub advisories_git_fetch_with_cli: Option<ScalarAssertion<bool>>,
    pub advisories_unused_ignored_advisory: Option<ScalarAssertion<value::DenyLintLevel>>,
    pub licenses_version: Option<ScalarAssertion<u64>>,
    pub licenses_include_dev: Option<ScalarAssertion<bool>>,
    pub licenses_include_build: Option<ScalarAssertion<bool>>,
    pub licenses_allow: ListRequirements,
    pub licenses_exceptions: ItemRequirements<value::DenyLicenseException>,
    pub licenses_confidence_threshold: Option<ScalarAssertion<value::DenyConfidenceThreshold>>,
    pub licenses_clarify: ItemRequirements<value::DenyLicenseClarification>,
    pub licenses_private_ignore: Option<ScalarAssertion<bool>>,
    pub licenses_private_registries: ListRequirements,
    pub licenses_private_ignore_sources: ListRequirements,
    pub licenses_unused_allowed_license: Option<ScalarAssertion<value::DenyLintLevel>>,
    pub licenses_unused_license_exception: Option<ScalarAssertion<value::DenyLintLevel>>,
    pub bans_multiple_versions: Option<ScalarAssertion<value::DenyLintLevel>>,
    pub bans_multiple_versions_include_dev: Option<ScalarAssertion<bool>>,
    pub bans_wildcards: Option<ScalarAssertion<value::DenyLintLevel>>,
    pub bans_allow_wildcard_paths: Option<ScalarAssertion<bool>>,
    pub bans_highlight: Option<ScalarAssertion<value::DenyGraphHighlight>>,
    pub bans_workspace_default_features: Option<ScalarAssertion<value::DenyLintLevel>>,
    pub bans_external_default_features: Option<ScalarAssertion<value::DenyLintLevel>>,
    pub bans_allow: ItemRequirements<value::DenyPackageReasonSpec>,
    pub bans_allow_workspace: Option<ScalarAssertion<bool>>,
    pub bans_deny: ItemRequirements<value::DenyBanSpec>,
    pub bans_features: ItemRequirements<value::DenyFeatureBanSpec>,
    pub bans_skip: ItemRequirements<value::DenyPackageReasonSpec>,
    pub bans_skip_tree: ItemRequirements<value::DenySkipTreeSpec>,
    pub bans_workspace_dependencies_duplicates: Option<ScalarAssertion<value::DenyLintLevel>>,
    pub bans_workspace_dependencies_include_path_dependencies: Option<ScalarAssertion<bool>>,
    pub bans_workspace_dependencies_unused: Option<ScalarAssertion<value::DenyLintLevel>>,
    pub bans_build_executables: Option<ScalarAssertion<value::DenyLintLevel>>,
    pub bans_build_interpreted: Option<ScalarAssertion<value::DenyLintLevel>>,
    pub bans_build_script_extensions: ListRequirements,
    pub bans_build_enable_builtin_globs: Option<ScalarAssertion<bool>>,
    pub bans_build_globs: ItemRequirements<value::DenyBuildGlobSpec>,
    pub bans_build_include_dependencies: Option<ScalarAssertion<bool>>,
    pub bans_build_include_workspace: Option<ScalarAssertion<bool>>,
    pub bans_build_include_archives: Option<ScalarAssertion<bool>>,
    pub sources_unknown_registry: Option<ScalarAssertion<value::DenyLintLevel>>,
    pub sources_unknown_git: Option<ScalarAssertion<value::DenyLintLevel>>,
    pub sources_required_git_spec: Option<ScalarAssertion<value::DenyGitSpec>>,
    pub sources_allow_git: ListRequirements,
    pub sources_private: ListRequirements,
    pub sources_allow_registry: ListRequirements,
    pub sources_allow_org_github: ListRequirements,
    pub sources_allow_org_gitlab: ListRequirements,
    pub sources_allow_org_bitbucket: ListRequirements,
    pub sources_unused_allowed_source: Option<ScalarAssertion<value::DenyLintLevel>>,
    pub table_keys: BTreeMap<DenyTable, ItemRequirements<KeyedItem<()>>>,
}

#[derive(Debug, Clone, Default)]
pub struct ResolvedDenyTomlRequirements {
    pub(crate) graph_targets: ResolvedItemRequirements<value::DenyGraphTargetSpec>,
    pub(crate) graph_exclude: ResolvedListRequirements,
    pub(crate) graph_exclude_dev: ResolvedScalar<bool>,
    pub(crate) graph_exclude_unpublished: ResolvedScalar<bool>,
    pub(crate) graph_all_features: ResolvedScalar<bool>,
    pub(crate) graph_no_default_features: ResolvedScalar<bool>,
    pub(crate) graph_features: ResolvedListRequirements,
    pub(crate) output_feature_depth: ResolvedScalar<u64>,
    pub(crate) advisories_version: ResolvedScalar<u64>,
    pub(crate) advisories_db_path: ResolvedScalar<value::DenyNonEmptyString>,
    pub(crate) advisories_db_urls: ResolvedListRequirements,
    pub(crate) advisories_yanked: ResolvedScalar<value::DenyLintLevel>,
    pub(crate) advisories_disable_yank_checking: ResolvedScalar<bool>,
    pub(crate) advisories_ignore: ResolvedItemRequirements<value::DenyAdvisoryIgnoreSpec>,
    pub(crate) advisories_unmaintained: ResolvedScalar<value::DenyAdvisoryScope>,
    pub(crate) advisories_unsound: ResolvedScalar<value::DenyAdvisoryScope>,
    pub(crate) advisories_maximum_db_staleness: ResolvedScalar<value::DenyDuration>,
    pub(crate) advisories_git_fetch_with_cli: ResolvedScalar<bool>,
    pub(crate) advisories_unused_ignored_advisory: ResolvedScalar<value::DenyLintLevel>,
    pub(crate) licenses_version: ResolvedScalar<u64>,
    pub(crate) licenses_include_dev: ResolvedScalar<bool>,
    pub(crate) licenses_include_build: ResolvedScalar<bool>,
    pub(crate) licenses_allow: ResolvedListRequirements,
    pub(crate) licenses_exceptions: ResolvedItemRequirements<value::DenyLicenseException>,
    pub(crate) licenses_confidence_threshold: ResolvedScalar<value::DenyConfidenceThreshold>,
    pub(crate) licenses_clarify: ResolvedItemRequirements<value::DenyLicenseClarification>,
    pub(crate) licenses_private_ignore: ResolvedScalar<bool>,
    pub(crate) licenses_private_registries: ResolvedListRequirements,
    pub(crate) licenses_private_ignore_sources: ResolvedListRequirements,
    pub(crate) licenses_unused_allowed_license: ResolvedScalar<value::DenyLintLevel>,
    pub(crate) licenses_unused_license_exception: ResolvedScalar<value::DenyLintLevel>,
    pub(crate) bans_multiple_versions: ResolvedScalar<value::DenyLintLevel>,
    pub(crate) bans_multiple_versions_include_dev: ResolvedScalar<bool>,
    pub(crate) bans_wildcards: ResolvedScalar<value::DenyLintLevel>,
    pub(crate) bans_allow_wildcard_paths: ResolvedScalar<bool>,
    pub(crate) bans_highlight: ResolvedScalar<value::DenyGraphHighlight>,
    pub(crate) bans_workspace_default_features: ResolvedScalar<value::DenyLintLevel>,
    pub(crate) bans_external_default_features: ResolvedScalar<value::DenyLintLevel>,
    pub(crate) bans_allow: ResolvedItemRequirements<value::DenyPackageReasonSpec>,
    pub(crate) bans_allow_workspace: ResolvedScalar<bool>,
    pub(crate) bans_deny: ResolvedItemRequirements<value::DenyBanSpec>,
    pub(crate) bans_features: ResolvedItemRequirements<value::DenyFeatureBanSpec>,
    pub(crate) bans_skip: ResolvedItemRequirements<value::DenyPackageReasonSpec>,
    pub(crate) bans_skip_tree: ResolvedItemRequirements<value::DenySkipTreeSpec>,
    pub(crate) bans_workspace_dependencies_duplicates: ResolvedScalar<value::DenyLintLevel>,
    pub(crate) bans_workspace_dependencies_include_path_dependencies: ResolvedScalar<bool>,
    pub(crate) bans_workspace_dependencies_unused: ResolvedScalar<value::DenyLintLevel>,
    pub(crate) bans_build_executables: ResolvedScalar<value::DenyLintLevel>,
    pub(crate) bans_build_interpreted: ResolvedScalar<value::DenyLintLevel>,
    pub(crate) bans_build_script_extensions: ResolvedListRequirements,
    pub(crate) bans_build_enable_builtin_globs: ResolvedScalar<bool>,
    pub(crate) bans_build_globs: ResolvedItemRequirements<value::DenyBuildGlobSpec>,
    pub(crate) bans_build_include_dependencies: ResolvedScalar<bool>,
    pub(crate) bans_build_include_workspace: ResolvedScalar<bool>,
    pub(crate) bans_build_include_archives: ResolvedScalar<bool>,
    pub(crate) sources_unknown_registry: ResolvedScalar<value::DenyLintLevel>,
    pub(crate) sources_unknown_git: ResolvedScalar<value::DenyLintLevel>,
    pub(crate) sources_required_git_spec: ResolvedScalar<value::DenyGitSpec>,
    pub(crate) sources_allow_git: ResolvedListRequirements,
    pub(crate) sources_private: ResolvedListRequirements,
    pub(crate) sources_allow_registry: ResolvedListRequirements,
    pub(crate) sources_allow_org_github: ResolvedListRequirements,
    pub(crate) sources_allow_org_gitlab: ResolvedListRequirements,
    pub(crate) sources_allow_org_bitbucket: ResolvedListRequirements,
    pub(crate) sources_unused_allowed_source: ResolvedScalar<value::DenyLintLevel>,
    pub(crate) table_keys: BTreeMap<DenyTable, ResolvedItemRequirements<KeyedItem<()>>>,
}

impl ResolvedDenyTomlRequirements {
    #[must_use]
    pub const fn graph_targets(&self) -> &ResolvedItemRequirements<value::DenyGraphTargetSpec> {
        &self.graph_targets
    }

    #[must_use]
    pub const fn graph_exclude(&self) -> &ResolvedListRequirements {
        &self.graph_exclude
    }

    #[must_use]
    pub const fn graph_exclude_dev(&self) -> ResolvedScalarRef<'_, bool> {
        self.graph_exclude_dev.as_ref()
    }

    #[must_use]
    pub const fn graph_exclude_unpublished(&self) -> ResolvedScalarRef<'_, bool> {
        self.graph_exclude_unpublished.as_ref()
    }

    #[must_use]
    pub const fn graph_all_features(&self) -> ResolvedScalarRef<'_, bool> {
        self.graph_all_features.as_ref()
    }

    #[must_use]
    pub const fn graph_no_default_features(&self) -> ResolvedScalarRef<'_, bool> {
        self.graph_no_default_features.as_ref()
    }

    #[must_use]
    pub const fn graph_features(&self) -> &ResolvedListRequirements {
        &self.graph_features
    }

    #[must_use]
    pub const fn output_feature_depth(&self) -> ResolvedScalarRef<'_, u64> {
        self.output_feature_depth.as_ref()
    }

    #[must_use]
    pub const fn advisories_version(&self) -> ResolvedScalarRef<'_, u64> {
        self.advisories_version.as_ref()
    }

    #[must_use]
    pub const fn advisories_db_path(&self) -> ResolvedScalarRef<'_, value::DenyNonEmptyString> {
        self.advisories_db_path.as_ref()
    }

    #[must_use]
    pub const fn advisories_db_urls(&self) -> &ResolvedListRequirements {
        &self.advisories_db_urls
    }

    #[must_use]
    pub const fn advisories_yanked(&self) -> ResolvedScalarRef<'_, value::DenyLintLevel> {
        self.advisories_yanked.as_ref()
    }

    #[must_use]
    pub const fn advisories_disable_yank_checking(&self) -> ResolvedScalarRef<'_, bool> {
        self.advisories_disable_yank_checking.as_ref()
    }

    #[must_use]
    pub const fn advisories_ignore(
        &self,
    ) -> &ResolvedItemRequirements<value::DenyAdvisoryIgnoreSpec> {
        &self.advisories_ignore
    }

    #[must_use]
    pub const fn advisories_unmaintained(&self) -> ResolvedScalarRef<'_, value::DenyAdvisoryScope> {
        self.advisories_unmaintained.as_ref()
    }

    #[must_use]
    pub const fn advisories_unsound(&self) -> ResolvedScalarRef<'_, value::DenyAdvisoryScope> {
        self.advisories_unsound.as_ref()
    }

    #[must_use]
    pub const fn advisories_maximum_db_staleness(
        &self,
    ) -> ResolvedScalarRef<'_, value::DenyDuration> {
        self.advisories_maximum_db_staleness.as_ref()
    }

    #[must_use]
    pub const fn advisories_git_fetch_with_cli(&self) -> ResolvedScalarRef<'_, bool> {
        self.advisories_git_fetch_with_cli.as_ref()
    }

    #[must_use]
    pub const fn advisories_unused_ignored_advisory(
        &self,
    ) -> ResolvedScalarRef<'_, value::DenyLintLevel> {
        self.advisories_unused_ignored_advisory.as_ref()
    }

    #[must_use]
    pub const fn licenses_version(&self) -> ResolvedScalarRef<'_, u64> {
        self.licenses_version.as_ref()
    }

    #[must_use]
    pub const fn licenses_include_dev(&self) -> ResolvedScalarRef<'_, bool> {
        self.licenses_include_dev.as_ref()
    }

    #[must_use]
    pub const fn licenses_include_build(&self) -> ResolvedScalarRef<'_, bool> {
        self.licenses_include_build.as_ref()
    }

    #[must_use]
    pub const fn licenses_allow(&self) -> &ResolvedListRequirements {
        &self.licenses_allow
    }

    #[must_use]
    pub const fn licenses_exceptions(
        &self,
    ) -> &ResolvedItemRequirements<value::DenyLicenseException> {
        &self.licenses_exceptions
    }

    #[must_use]
    pub const fn licenses_confidence_threshold(
        &self,
    ) -> ResolvedScalarRef<'_, value::DenyConfidenceThreshold> {
        self.licenses_confidence_threshold.as_ref()
    }

    #[must_use]
    pub const fn licenses_clarify(
        &self,
    ) -> &ResolvedItemRequirements<value::DenyLicenseClarification> {
        &self.licenses_clarify
    }

    #[must_use]
    pub const fn licenses_private_ignore(&self) -> ResolvedScalarRef<'_, bool> {
        self.licenses_private_ignore.as_ref()
    }

    #[must_use]
    pub const fn licenses_private_registries(&self) -> &ResolvedListRequirements {
        &self.licenses_private_registries
    }

    #[must_use]
    pub const fn licenses_private_ignore_sources(&self) -> &ResolvedListRequirements {
        &self.licenses_private_ignore_sources
    }

    #[must_use]
    pub const fn licenses_unused_allowed_license(
        &self,
    ) -> ResolvedScalarRef<'_, value::DenyLintLevel> {
        self.licenses_unused_allowed_license.as_ref()
    }

    #[must_use]
    pub const fn licenses_unused_license_exception(
        &self,
    ) -> ResolvedScalarRef<'_, value::DenyLintLevel> {
        self.licenses_unused_license_exception.as_ref()
    }

    #[must_use]
    pub const fn bans_multiple_versions(&self) -> ResolvedScalarRef<'_, value::DenyLintLevel> {
        self.bans_multiple_versions.as_ref()
    }

    #[must_use]
    pub const fn bans_multiple_versions_include_dev(&self) -> ResolvedScalarRef<'_, bool> {
        self.bans_multiple_versions_include_dev.as_ref()
    }

    #[must_use]
    pub const fn bans_wildcards(&self) -> ResolvedScalarRef<'_, value::DenyLintLevel> {
        self.bans_wildcards.as_ref()
    }

    #[must_use]
    pub const fn bans_allow_wildcard_paths(&self) -> ResolvedScalarRef<'_, bool> {
        self.bans_allow_wildcard_paths.as_ref()
    }

    #[must_use]
    pub const fn bans_highlight(&self) -> ResolvedScalarRef<'_, value::DenyGraphHighlight> {
        self.bans_highlight.as_ref()
    }

    #[must_use]
    pub const fn bans_workspace_default_features(
        &self,
    ) -> ResolvedScalarRef<'_, value::DenyLintLevel> {
        self.bans_workspace_default_features.as_ref()
    }

    #[must_use]
    pub const fn bans_external_default_features(
        &self,
    ) -> ResolvedScalarRef<'_, value::DenyLintLevel> {
        self.bans_external_default_features.as_ref()
    }

    #[must_use]
    pub const fn bans_allow(&self) -> &ResolvedItemRequirements<value::DenyPackageReasonSpec> {
        &self.bans_allow
    }

    #[must_use]
    pub const fn bans_allow_workspace(&self) -> ResolvedScalarRef<'_, bool> {
        self.bans_allow_workspace.as_ref()
    }

    #[must_use]
    pub const fn bans_deny(&self) -> &ResolvedItemRequirements<value::DenyBanSpec> {
        &self.bans_deny
    }

    #[must_use]
    pub const fn bans_features(&self) -> &ResolvedItemRequirements<value::DenyFeatureBanSpec> {
        &self.bans_features
    }

    #[must_use]
    pub const fn bans_skip(&self) -> &ResolvedItemRequirements<value::DenyPackageReasonSpec> {
        &self.bans_skip
    }

    #[must_use]
    pub const fn bans_skip_tree(&self) -> &ResolvedItemRequirements<value::DenySkipTreeSpec> {
        &self.bans_skip_tree
    }

    #[must_use]
    pub const fn bans_workspace_dependencies_duplicates(
        &self,
    ) -> ResolvedScalarRef<'_, value::DenyLintLevel> {
        self.bans_workspace_dependencies_duplicates.as_ref()
    }

    #[must_use]
    pub const fn bans_workspace_dependencies_include_path_dependencies(
        &self,
    ) -> ResolvedScalarRef<'_, bool> {
        self.bans_workspace_dependencies_include_path_dependencies
            .as_ref()
    }

    #[must_use]
    pub const fn bans_workspace_dependencies_unused(
        &self,
    ) -> ResolvedScalarRef<'_, value::DenyLintLevel> {
        self.bans_workspace_dependencies_unused.as_ref()
    }

    #[must_use]
    pub const fn bans_build_executables(&self) -> ResolvedScalarRef<'_, value::DenyLintLevel> {
        self.bans_build_executables.as_ref()
    }

    #[must_use]
    pub const fn bans_build_interpreted(&self) -> ResolvedScalarRef<'_, value::DenyLintLevel> {
        self.bans_build_interpreted.as_ref()
    }

    #[must_use]
    pub const fn bans_build_script_extensions(&self) -> &ResolvedListRequirements {
        &self.bans_build_script_extensions
    }

    #[must_use]
    pub const fn bans_build_enable_builtin_globs(&self) -> ResolvedScalarRef<'_, bool> {
        self.bans_build_enable_builtin_globs.as_ref()
    }

    #[must_use]
    pub const fn bans_build_globs(&self) -> &ResolvedItemRequirements<value::DenyBuildGlobSpec> {
        &self.bans_build_globs
    }

    #[must_use]
    pub const fn bans_build_include_dependencies(&self) -> ResolvedScalarRef<'_, bool> {
        self.bans_build_include_dependencies.as_ref()
    }

    #[must_use]
    pub const fn bans_build_include_workspace(&self) -> ResolvedScalarRef<'_, bool> {
        self.bans_build_include_workspace.as_ref()
    }

    #[must_use]
    pub const fn bans_build_include_archives(&self) -> ResolvedScalarRef<'_, bool> {
        self.bans_build_include_archives.as_ref()
    }

    #[must_use]
    pub const fn sources_unknown_registry(&self) -> ResolvedScalarRef<'_, value::DenyLintLevel> {
        self.sources_unknown_registry.as_ref()
    }

    #[must_use]
    pub const fn sources_unknown_git(&self) -> ResolvedScalarRef<'_, value::DenyLintLevel> {
        self.sources_unknown_git.as_ref()
    }

    #[must_use]
    pub const fn sources_required_git_spec(&self) -> ResolvedScalarRef<'_, value::DenyGitSpec> {
        self.sources_required_git_spec.as_ref()
    }

    #[must_use]
    pub const fn sources_allow_git(&self) -> &ResolvedListRequirements {
        &self.sources_allow_git
    }

    #[must_use]
    pub const fn sources_private(&self) -> &ResolvedListRequirements {
        &self.sources_private
    }

    #[must_use]
    pub const fn sources_allow_registry(&self) -> &ResolvedListRequirements {
        &self.sources_allow_registry
    }

    #[must_use]
    pub const fn sources_allow_org_github(&self) -> &ResolvedListRequirements {
        &self.sources_allow_org_github
    }

    #[must_use]
    pub const fn sources_allow_org_gitlab(&self) -> &ResolvedListRequirements {
        &self.sources_allow_org_gitlab
    }

    #[must_use]
    pub const fn sources_allow_org_bitbucket(&self) -> &ResolvedListRequirements {
        &self.sources_allow_org_bitbucket
    }

    #[must_use]
    pub const fn sources_unused_allowed_source(
        &self,
    ) -> ResolvedScalarRef<'_, value::DenyLintLevel> {
        self.sources_unused_allowed_source.as_ref()
    }

    #[must_use]
    pub const fn table_keys(
        &self,
    ) -> &BTreeMap<DenyTable, ResolvedItemRequirements<KeyedItem<()>>> {
        &self.table_keys
    }
}

impl EngineRequirement for DenyTomlRequirements {
    fn engine_id(&self) -> &'static str {
        crate::ENGINE_ID
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
