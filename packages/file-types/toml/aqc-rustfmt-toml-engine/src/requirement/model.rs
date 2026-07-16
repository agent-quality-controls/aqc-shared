//! Rustfmt requirement model types.

#![allow(
    clippy::disallowed_types,
    reason = "`Any` is used only for EngineRequirement downcast dispatch."
)]

use core::any::Any;
use std::collections::BTreeMap;

use aqc_file_engine_core::{
    ConfigScalar, EngineRequirement, ForbiddenGlobRequirement, ForbiddenGlobRequirements,
    ItemRequirements, KeyedItem, ListRequirements, ResolvedForbiddenGlobRequirements,
    ResolvedItemRequirements, ResolvedListRequirements, ResolvedRequirement, ScalarAssertion,
};

use super::settings::{RustfmtListSetting, RustfmtScalarSetting};

/// Resolved scalar settings keyed by rustfmt setting name.
pub type ResolvedRustfmtScalarSettings = BTreeMap<
    RustfmtScalarSetting,
    ResolvedRequirement<ScalarAssertion<ConfigScalar>, ScalarAssertion<ConfigScalar>>,
>;

/// Raw scalar setting requirements keyed by rustfmt setting name.
pub type RustfmtScalarRequirements = BTreeMap<RustfmtScalarSetting, ScalarAssertion<ConfigScalar>>;

#[derive(Debug, Clone, Default)]
pub struct RustfmtTomlRequirements {
    pub scalar_settings: RustfmtScalarRequirements,
    pub list_settings: BTreeMap<RustfmtListSetting, ListRequirements>,
    pub forbidden_ignore_path_globs: ForbiddenGlobRequirements<RustfmtIgnorePathGlob>,
    pub setting_keys: ItemRequirements<KeyedItem<()>>,
}

#[derive(Debug, Clone, Default)]
pub struct ResolvedRustfmtTomlRequirements {
    pub(crate) scalar_settings: ResolvedRustfmtScalarSettings,
    pub(crate) list_settings: BTreeMap<RustfmtListSetting, ResolvedListRequirements>,
    pub(crate) forbidden_ignore_path_globs:
        ResolvedForbiddenGlobRequirements<RustfmtIgnorePathGlob>,
    pub(crate) setting_keys: ResolvedItemRequirements<KeyedItem<()>>,
}

impl ResolvedRustfmtTomlRequirements {
    #[must_use]
    pub const fn scalar_settings(&self) -> &ResolvedRustfmtScalarSettings {
        &self.scalar_settings
    }

    #[must_use]
    pub const fn list_settings(&self) -> &BTreeMap<RustfmtListSetting, ResolvedListRequirements> {
        &self.list_settings
    }

    #[rustfmt::skip]
    #[must_use]
    pub const fn forbidden_ignore_path_globs(&self) -> &ResolvedForbiddenGlobRequirements<RustfmtIgnorePathGlob> {
        &self.forbidden_ignore_path_globs
    }

    #[must_use]
    pub const fn setting_keys(&self) -> &ResolvedItemRequirements<KeyedItem<()>> {
        &self.setting_keys
    }
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
