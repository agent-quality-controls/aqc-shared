//! Clippy requirement aggregate types.

#![expect(
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

use super::{ClippyForbiddenGlobConflictBlocks, ClippyPathGlob, DisallowedEntry};

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
    pub msrv:
        Option<ResolvedRequirement<ScalarAssertion<DottedVersion>, ScalarAssertion<DottedVersion>>>,
    pub thresholds:
        BTreeMap<String, ResolvedRequirement<ScalarAssertion<u64>, ScalarAssertion<u64>>>,
    pub disallowed_methods: ResolvedItemRequirements<DisallowedEntry>,
    pub forbidden_disallowed_method_path_globs: ResolvedForbiddenGlobRequirements<ClippyPathGlob>,
    pub disallowed_method_glob_conflicts: ClippyForbiddenGlobConflictBlocks,
    pub disallowed_types: ResolvedItemRequirements<DisallowedEntry>,
    pub forbidden_disallowed_type_path_globs: ResolvedForbiddenGlobRequirements<ClippyPathGlob>,
    pub disallowed_type_glob_conflicts: ClippyForbiddenGlobConflictBlocks,
    pub disallowed_macros: ResolvedItemRequirements<DisallowedEntry>,
    pub forbidden_disallowed_macro_path_globs: ResolvedForbiddenGlobRequirements<ClippyPathGlob>,
    pub disallowed_macro_glob_conflicts: ClippyForbiddenGlobConflictBlocks,
    pub bools: BTreeMap<String, ResolvedRequirement<ScalarAssertion<bool>, ScalarAssertion<bool>>>,
    pub enums:
        BTreeMap<String, ResolvedRequirement<ScalarAssertion<String>, ScalarAssertion<String>>>,
}

impl EngineRequirement for ClippyTomlRequirements {
    fn engine_id(&self) -> &'static str {
        crate::ENGINE_ID
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
