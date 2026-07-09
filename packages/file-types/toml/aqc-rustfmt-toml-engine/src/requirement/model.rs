//! Rustfmt requirement model types.

#![allow(
    clippy::disallowed_types,
    reason = "`Any` is used only for EngineRequirement downcast dispatch."
)]

use core::any::Any;
use std::collections::{BTreeMap, BTreeSet};

use aqc_file_engine_core::{
    ConfigScalar, EngineRequirement, ForbiddenGlobRequirement, ForbiddenGlobRequirements,
    ListRequirements, Provenance, ResolvedForbiddenGlobRequirements, ResolvedListRequirements,
    ResolvedRequirement, ScalarAssertion,
};

use super::settings::{RustfmtListSetting, RustfmtScalarSetting};

/// Resolved scalar settings keyed by rustfmt setting name.
pub type ResolvedRustfmtScalarSettings = BTreeMap<
    RustfmtScalarSetting,
    ResolvedRequirement<ScalarAssertion<ConfigScalar>, ScalarAssertion<ConfigScalar>>,
>;

/// Policy provenance entries that closed the rustfmt setting set.
pub type ResolvedRustfmtClosedSettings = Vec<(Provenance, String)>;

/// Raw scalar setting requirements keyed by rustfmt setting name.
pub type RustfmtScalarRequirements = BTreeMap<RustfmtScalarSetting, ScalarAssertion<ConfigScalar>>;

#[derive(Debug, Clone, Default)]
pub struct RustfmtTomlRequirements {
    pub scalar_settings: RustfmtScalarRequirements,
    pub list_settings: BTreeMap<RustfmtListSetting, ListRequirements>,
    pub forbidden_ignore_path_globs: ForbiddenGlobRequirements<RustfmtIgnorePathGlob>,
    pub closed_settings: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct ResolvedRustfmtTomlRequirements {
    pub scalar_settings: ResolvedRustfmtScalarSettings,
    pub list_settings: BTreeMap<RustfmtListSetting, ResolvedListRequirements>,
    pub forbidden_ignore_path_globs: ResolvedForbiddenGlobRequirements<RustfmtIgnorePathGlob>,
    pub ignore_glob_conflicts: RustfmtForbiddenIgnoreGlobConflictBlocks,
    pub closed_settings: ResolvedRustfmtClosedSettings,
}

impl EngineRequirement for RustfmtTomlRequirements {
    fn engine_id(&self) -> &'static str {
        crate::ENGINE_ID
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// Path glob used only for forbidden `ignore` entries.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RustfmtIgnorePathGlob {
    pub glob: String,
}

impl ForbiddenGlobRequirement for RustfmtIgnorePathGlob {
    type Identity = String;

    fn merge_identity(&self) -> Self::Identity {
        self.glob.clone()
    }

    fn render(&self) -> String {
        self.glob.clone()
    }
}

/// Required `ignore` values and forbidden `ignore` globs that conflict.
#[derive(Debug, Clone, Default)]
pub struct RustfmtForbiddenIgnoreGlobConflictBlocks {
    /// Required `ignore` values blocked during reconciliation.
    pub required: BTreeSet<String>,
    /// Forbidden `ignore` globs blocked during reconciliation.
    pub path_globs: BTreeSet<String>,
}
