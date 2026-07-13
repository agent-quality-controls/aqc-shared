//! Clippy requirement aggregate types.

#![allow(
    clippy::disallowed_types,
    clippy::type_complexity,
    reason = "`Any` is required by the shared EngineRequirement downcast API; resolved requirement fields mirror public API sections."
)]

use core::any::Any;
use std::collections::BTreeMap;

use aqc_file_engine_core::{
    DottedVersion, EngineRequirement, ForbiddenGlobRequirements, ItemRequirements,
    ResolvedForbiddenGlobRequirements, ResolvedItemRequirements, ResolvedRequirement,
    ScalarAssertion,
};

use super::{ClippyPathGlob, DisallowedEntry};

#[derive(Debug, Clone, Default)]
pub struct ClippyTomlRequirements {
    pub msrv: Option<ScalarAssertion<DottedVersion>>,
    pub thresholds: BTreeMap<String, ScalarAssertion<u64>>,
    pub disallowed_methods: ItemRequirements<DisallowedEntry>,
    pub forbidden_disallowed_method_path_globs: ForbiddenGlobRequirements<ClippyPathGlob>,
    pub disallowed_types: ItemRequirements<DisallowedEntry>,
    pub forbidden_disallowed_type_path_globs: ForbiddenGlobRequirements<ClippyPathGlob>,
    pub disallowed_macros: ItemRequirements<DisallowedEntry>,
    pub forbidden_disallowed_macro_path_globs: ForbiddenGlobRequirements<ClippyPathGlob>,
    pub bools: BTreeMap<String, ScalarAssertion<bool>>,
    pub enums: BTreeMap<String, ScalarAssertion<String>>,
}

#[derive(Debug, Clone, Default)]
pub struct ResolvedClippyTomlRequirements {
    pub(crate) msrv:
        Option<ResolvedRequirement<ScalarAssertion<DottedVersion>, ScalarAssertion<DottedVersion>>>,
    pub(crate) thresholds:
        BTreeMap<String, ResolvedRequirement<ScalarAssertion<u64>, ScalarAssertion<u64>>>,
    pub(crate) disallowed_methods: ResolvedItemRequirements<DisallowedEntry>,
    pub(crate) forbidden_disallowed_method_path_globs:
        ResolvedForbiddenGlobRequirements<ClippyPathGlob>,
    pub(crate) disallowed_types: ResolvedItemRequirements<DisallowedEntry>,
    pub(crate) forbidden_disallowed_type_path_globs:
        ResolvedForbiddenGlobRequirements<ClippyPathGlob>,
    pub(crate) disallowed_macros: ResolvedItemRequirements<DisallowedEntry>,
    pub(crate) forbidden_disallowed_macro_path_globs:
        ResolvedForbiddenGlobRequirements<ClippyPathGlob>,
    pub(crate) bools:
        BTreeMap<String, ResolvedRequirement<ScalarAssertion<bool>, ScalarAssertion<bool>>>,
    pub(crate) enums:
        BTreeMap<String, ResolvedRequirement<ScalarAssertion<String>, ScalarAssertion<String>>>,
}

impl ResolvedClippyTomlRequirements {
    #[must_use]
    pub const fn msrv(
        &self,
    ) -> Option<&ResolvedRequirement<ScalarAssertion<DottedVersion>, ScalarAssertion<DottedVersion>>>
    {
        self.msrv.as_ref()
    }

    #[must_use]
    pub const fn thresholds(
        &self,
    ) -> &BTreeMap<String, ResolvedRequirement<ScalarAssertion<u64>, ScalarAssertion<u64>>> {
        &self.thresholds
    }

    #[must_use]
    pub const fn disallowed_methods(&self) -> &ResolvedItemRequirements<DisallowedEntry> {
        &self.disallowed_methods
    }

    #[must_use]
    pub const fn forbidden_disallowed_method_path_globs(
        &self,
    ) -> &ResolvedForbiddenGlobRequirements<ClippyPathGlob> {
        &self.forbidden_disallowed_method_path_globs
    }

    #[must_use]
    pub const fn disallowed_types(&self) -> &ResolvedItemRequirements<DisallowedEntry> {
        &self.disallowed_types
    }

    #[must_use]
    pub const fn forbidden_disallowed_type_path_globs(
        &self,
    ) -> &ResolvedForbiddenGlobRequirements<ClippyPathGlob> {
        &self.forbidden_disallowed_type_path_globs
    }

    #[must_use]
    pub const fn disallowed_macros(&self) -> &ResolvedItemRequirements<DisallowedEntry> {
        &self.disallowed_macros
    }

    #[must_use]
    pub const fn forbidden_disallowed_macro_path_globs(
        &self,
    ) -> &ResolvedForbiddenGlobRequirements<ClippyPathGlob> {
        &self.forbidden_disallowed_macro_path_globs
    }

    #[must_use]
    pub const fn bools(
        &self,
    ) -> &BTreeMap<String, ResolvedRequirement<ScalarAssertion<bool>, ScalarAssertion<bool>>> {
        &self.bools
    }

    #[must_use]
    pub const fn enums(
        &self,
    ) -> &BTreeMap<String, ResolvedRequirement<ScalarAssertion<String>, ScalarAssertion<String>>>
    {
        &self.enums
    }
}

impl EngineRequirement for ClippyTomlRequirements {
    fn engine_id(&self) -> &'static str {
        crate::ENGINE_ID
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
