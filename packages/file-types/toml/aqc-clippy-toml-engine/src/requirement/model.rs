//! Clippy requirement aggregate types.

use core::any::Any;
use std::collections::BTreeMap;

use aqc_file_engine_core::{
    EngineRequirement, ForbiddenGlobRequirements, ItemRequirements,
    ResolvedForbiddenGlobRequirements, ResolvedItemRequirements, ResolvedRequirement,
};

use super::{
    BanEntry, BoolAssertion, ClippyForbiddenGlobConflictBlocks, ClippyPathGlob, MsrvAssertion,
    NumericAssertion, StringAssertion,
};

#[derive(Debug, Clone, Default)]
pub struct ClippyTomlRequirements {
    pub msrv: Option<MsrvAssertion>,
    pub thresholds: BTreeMap<String, NumericAssertion>,
    pub disallowed_methods: ItemRequirements<BanEntry>,
    pub forbidden_disallowed_method_path_globs: ForbiddenGlobRequirements<ClippyPathGlob>,
    pub disallowed_types: ItemRequirements<BanEntry>,
    pub forbidden_disallowed_type_path_globs: ForbiddenGlobRequirements<ClippyPathGlob>,
    pub disallowed_macros: ItemRequirements<BanEntry>,
    pub forbidden_disallowed_macro_path_globs: ForbiddenGlobRequirements<ClippyPathGlob>,
    pub bools: BTreeMap<String, BoolAssertion>,
    pub enums: BTreeMap<String, StringAssertion>,
}

#[derive(Debug, Clone, Default)]
pub struct ResolvedClippyTomlRequirements {
    pub msrv: Option<ResolvedRequirement<MsrvAssertion, MsrvAssertion>>,
    pub thresholds: BTreeMap<String, ResolvedRequirement<NumericAssertion, NumericAssertion>>,
    pub disallowed_methods: ResolvedItemRequirements<BanEntry>,
    pub forbidden_disallowed_method_path_globs: ResolvedForbiddenGlobRequirements<ClippyPathGlob>,
    pub disallowed_method_glob_conflicts: ClippyForbiddenGlobConflictBlocks,
    pub disallowed_types: ResolvedItemRequirements<BanEntry>,
    pub forbidden_disallowed_type_path_globs: ResolvedForbiddenGlobRequirements<ClippyPathGlob>,
    pub disallowed_type_glob_conflicts: ClippyForbiddenGlobConflictBlocks,
    pub disallowed_macros: ResolvedItemRequirements<BanEntry>,
    pub forbidden_disallowed_macro_path_globs: ResolvedForbiddenGlobRequirements<ClippyPathGlob>,
    pub disallowed_macro_glob_conflicts: ClippyForbiddenGlobConflictBlocks,
    pub bools: BTreeMap<String, ResolvedRequirement<BoolAssertion, BoolAssertion>>,
    pub enums: BTreeMap<String, ResolvedRequirement<StringAssertion, StringAssertion>>,
}

impl EngineRequirement for ClippyTomlRequirements {
    fn engine_id(&self) -> &'static str {
        crate::ENGINE_ID
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
