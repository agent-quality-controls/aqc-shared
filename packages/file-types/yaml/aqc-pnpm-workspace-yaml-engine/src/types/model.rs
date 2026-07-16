//! Unresolved and resolved pnpm workspace requirements.

#![allow(
    clippy::disallowed_types,
    reason = "Any supports engine requirement dispatch."
)]

use core::any::Any;

use aqc_file_engine_core::{
    EngineRequirement, ForbiddenGlobRequirement, ForbiddenGlobRequirements, ItemRequirements,
    KeyedItem, ListRequirements, ResolvedForbiddenGlobRequirements, ResolvedItemRequirements,
    ResolvedListRequirements, ResolvedRequirement, ScalarAssertion,
};

use crate::types::{PnpmOnFail, PnpmReleaseAgeMinutes, PnpmTrustPolicy};

pub const ENGINE_ID: &str = "aqc-pnpm-workspace-yaml-engine";

#[derive(Debug, Clone, Default)]
pub struct PnpmWorkspaceYamlRequirements {
    pub strict_peer_dependencies: Option<ScalarAssertion<bool>>,
    pub engine_strict: Option<ScalarAssertion<bool>>,
    pub minimum_release_age: Option<ScalarAssertion<PnpmReleaseAgeMinutes>>,
    pub minimum_release_age_strict: Option<ScalarAssertion<bool>>,
    pub minimum_release_age_ignore_missing_time: Option<ScalarAssertion<bool>>,
    pub minimum_release_age_exclude: ListRequirements,
    pub forbidden_minimum_release_age_exclude_globs:
        ForbiddenGlobRequirements<PnpmPackageSelectorGlob>,
    pub trust_policy: Option<ScalarAssertion<PnpmTrustPolicy>>,
    pub trust_lockfile: Option<ScalarAssertion<bool>>,
    pub trust_policy_ignore_after: Option<ScalarAssertion<u64>>,
    pub trust_policy_exclude: ListRequirements,
    pub forbidden_trust_policy_exclude_globs: ForbiddenGlobRequirements<PnpmPackageSelectorGlob>,
    pub block_exotic_subdeps: Option<ScalarAssertion<bool>>,
    pub pm_on_fail: Option<ScalarAssertion<PnpmOnFail>>,
    pub strict_dep_builds: Option<ScalarAssertion<bool>>,
    pub dangerously_allow_all_builds: Option<ScalarAssertion<bool>>,
    pub allow_builds: ItemRequirements<KeyedItem<bool>>,
    pub forbidden_allowed_build_package_globs: ForbiddenGlobRequirements<PnpmPackageSelectorGlob>,
    pub root_keys: ItemRequirements<KeyedItem<()>>,
}

#[derive(Debug, Clone)]
#[rustfmt::skip]
pub struct ResolvedPnpmWorkspaceYamlRequirements {
    pub(crate) strict_peer_dependencies: Option<ResolvedRequirement<ScalarAssertion<bool>, ScalarAssertion<bool>>>,
    pub(crate) engine_strict: Option<ResolvedRequirement<ScalarAssertion<bool>, ScalarAssertion<bool>>>,
    pub(crate) minimum_release_age: Option<ResolvedRequirement<ScalarAssertion<PnpmReleaseAgeMinutes>, ScalarAssertion<PnpmReleaseAgeMinutes>>>,
    pub(crate) minimum_release_age_strict: Option<ResolvedRequirement<ScalarAssertion<bool>, ScalarAssertion<bool>>>,
    pub(crate) minimum_release_age_ignore_missing_time: Option<ResolvedRequirement<ScalarAssertion<bool>, ScalarAssertion<bool>>>,
    pub(crate) minimum_release_age_exclude: ResolvedListRequirements,
    pub(crate) forbidden_minimum_release_age_exclude_globs: ResolvedForbiddenGlobRequirements<PnpmPackageSelectorGlob>,
    pub(crate) trust_policy: Option<ResolvedRequirement<ScalarAssertion<PnpmTrustPolicy>, ScalarAssertion<PnpmTrustPolicy>>>,
    pub(crate) trust_lockfile: Option<ResolvedRequirement<ScalarAssertion<bool>, ScalarAssertion<bool>>>,
    pub(crate) trust_policy_ignore_after: Option<ResolvedRequirement<ScalarAssertion<u64>, ScalarAssertion<u64>>>,
    pub(crate) trust_policy_exclude: ResolvedListRequirements,
    pub(crate) forbidden_trust_policy_exclude_globs: ResolvedForbiddenGlobRequirements<PnpmPackageSelectorGlob>,
    pub(crate) block_exotic_subdeps: Option<ResolvedRequirement<ScalarAssertion<bool>, ScalarAssertion<bool>>>,
    pub(crate) pm_on_fail: Option<ResolvedRequirement<ScalarAssertion<PnpmOnFail>, ScalarAssertion<PnpmOnFail>>>,
    pub(crate) strict_dep_builds: Option<ResolvedRequirement<ScalarAssertion<bool>, ScalarAssertion<bool>>>,
    pub(crate) dangerously_allow_all_builds: Option<ResolvedRequirement<ScalarAssertion<bool>, ScalarAssertion<bool>>>,
    pub(crate) allow_builds: ResolvedItemRequirements<KeyedItem<bool>>,
    pub(crate) forbidden_allowed_build_package_globs: ResolvedForbiddenGlobRequirements<PnpmPackageSelectorGlob>,
    pub(crate) root_keys: ResolvedItemRequirements<KeyedItem<()>>,
}

impl ResolvedPnpmWorkspaceYamlRequirements {
    #[must_use]
    pub const fn strict_peer_dependencies(
        &self,
    ) -> Option<&ResolvedRequirement<ScalarAssertion<bool>, ScalarAssertion<bool>>> {
        self.strict_peer_dependencies.as_ref()
    }
    #[must_use]
    pub const fn engine_strict(
        &self,
    ) -> Option<&ResolvedRequirement<ScalarAssertion<bool>, ScalarAssertion<bool>>> {
        self.engine_strict.as_ref()
    }
    #[must_use]
    pub const fn minimum_release_age(
        &self,
    ) -> Option<
        &ResolvedRequirement<
            ScalarAssertion<PnpmReleaseAgeMinutes>,
            ScalarAssertion<PnpmReleaseAgeMinutes>,
        >,
    > {
        self.minimum_release_age.as_ref()
    }
    #[must_use]
    pub const fn minimum_release_age_strict(
        &self,
    ) -> Option<&ResolvedRequirement<ScalarAssertion<bool>, ScalarAssertion<bool>>> {
        self.minimum_release_age_strict.as_ref()
    }
    #[must_use]
    pub const fn minimum_release_age_ignore_missing_time(
        &self,
    ) -> Option<&ResolvedRequirement<ScalarAssertion<bool>, ScalarAssertion<bool>>> {
        self.minimum_release_age_ignore_missing_time.as_ref()
    }
    #[must_use]
    pub const fn trust_policy(
        &self,
    ) -> Option<
        &ResolvedRequirement<ScalarAssertion<PnpmTrustPolicy>, ScalarAssertion<PnpmTrustPolicy>>,
    > {
        self.trust_policy.as_ref()
    }
    #[must_use]
    pub const fn trust_lockfile(
        &self,
    ) -> Option<&ResolvedRequirement<ScalarAssertion<bool>, ScalarAssertion<bool>>> {
        self.trust_lockfile.as_ref()
    }
    #[must_use]
    pub const fn trust_policy_ignore_after(
        &self,
    ) -> Option<&ResolvedRequirement<ScalarAssertion<u64>, ScalarAssertion<u64>>> {
        self.trust_policy_ignore_after.as_ref()
    }
    #[must_use]
    pub const fn block_exotic_subdeps(
        &self,
    ) -> Option<&ResolvedRequirement<ScalarAssertion<bool>, ScalarAssertion<bool>>> {
        self.block_exotic_subdeps.as_ref()
    }
    #[must_use]
    pub const fn pm_on_fail(
        &self,
    ) -> Option<&ResolvedRequirement<ScalarAssertion<PnpmOnFail>, ScalarAssertion<PnpmOnFail>>>
    {
        self.pm_on_fail.as_ref()
    }
    #[must_use]
    pub const fn strict_dep_builds(
        &self,
    ) -> Option<&ResolvedRequirement<ScalarAssertion<bool>, ScalarAssertion<bool>>> {
        self.strict_dep_builds.as_ref()
    }
    #[must_use]
    pub const fn dangerously_allow_all_builds(
        &self,
    ) -> Option<&ResolvedRequirement<ScalarAssertion<bool>, ScalarAssertion<bool>>> {
        self.dangerously_allow_all_builds.as_ref()
    }

    #[must_use]
    pub const fn minimum_release_age_exclude(&self) -> &ResolvedListRequirements {
        &self.minimum_release_age_exclude
    }
    #[must_use]
    pub const fn forbidden_minimum_release_age_exclude_globs(
        &self,
    ) -> &ResolvedForbiddenGlobRequirements<PnpmPackageSelectorGlob> {
        &self.forbidden_minimum_release_age_exclude_globs
    }
    #[must_use]
    pub const fn trust_policy_exclude(&self) -> &ResolvedListRequirements {
        &self.trust_policy_exclude
    }
    #[must_use]
    pub const fn forbidden_trust_policy_exclude_globs(
        &self,
    ) -> &ResolvedForbiddenGlobRequirements<PnpmPackageSelectorGlob> {
        &self.forbidden_trust_policy_exclude_globs
    }
    #[must_use]
    pub const fn allow_builds(&self) -> &ResolvedItemRequirements<KeyedItem<bool>> {
        &self.allow_builds
    }
    #[must_use]
    pub const fn forbidden_allowed_build_package_globs(
        &self,
    ) -> &ResolvedForbiddenGlobRequirements<PnpmPackageSelectorGlob> {
        &self.forbidden_allowed_build_package_globs
    }
    #[must_use]
    pub const fn root_keys(&self) -> &ResolvedItemRequirements<KeyedItem<()>> {
        &self.root_keys
    }
}

impl EngineRequirement for PnpmWorkspaceYamlRequirements {
    fn engine_id(&self) -> &'static str {
        ENGINE_ID
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PnpmPackageSelectorGlob {
    pub glob: String,
}

impl ForbiddenGlobRequirement for PnpmPackageSelectorGlob {
    type Identity = String;

    fn merge_identity(&self) -> Self::Identity {
        self.glob.clone()
    }

    fn render(&self) -> String {
        self.glob.clone()
    }
}
