//! Rust toolchain requirement model types.

#![expect(
    clippy::disallowed_types,
    reason = "`Any` is used only for EngineRequirement downcast dispatch."
)]

use core::any::Any;
use std::collections::BTreeMap;

use aqc_file_engine_core::{
    ConfigScalar, EngineRequirement, ListRequirements, Provenance, ResolvedListRequirements,
    ResolvedRequirement, ScalarAssertion,
};

use super::settings::{RustToolchainListSetting, RustToolchainScalarSetting};

pub type RustToolchainScalarSettings =
    BTreeMap<RustToolchainScalarSetting, ScalarAssertion<ConfigScalar>>;

pub type ResolvedRustToolchainScalarSettings = BTreeMap<
    RustToolchainScalarSetting,
    ResolvedRequirement<ScalarAssertion<ConfigScalar>, ScalarAssertion<ConfigScalar>>,
>;

pub type ResolvedRustToolchainClosedSettings = Vec<(Provenance, String)>;

#[derive(Debug, Clone, Default)]
pub struct RustToolchainTomlRequirements {
    pub scalar_settings: RustToolchainScalarSettings,
    pub list_settings: BTreeMap<RustToolchainListSetting, ListRequirements>,
    pub closed_settings: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct ResolvedRustToolchainTomlRequirements {
    pub scalar_settings: ResolvedRustToolchainScalarSettings,
    pub list_settings: BTreeMap<RustToolchainListSetting, ResolvedListRequirements>,
    pub closed_settings: ResolvedRustToolchainClosedSettings,
}

impl EngineRequirement for RustToolchainTomlRequirements {
    fn engine_id(&self) -> &'static str {
        crate::ENGINE_ID
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
