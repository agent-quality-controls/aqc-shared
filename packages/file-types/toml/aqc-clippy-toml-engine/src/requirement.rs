//! Declarative requirement and assertion types accepted by `ClippyTomlEngine`.

#![expect(
    clippy::disallowed_types,
    reason = "`Any` is used only in the `EngineRequirement::as_any` impl; the broker uses it to downcast `Box<dyn EngineRequirement>` back to this concrete `Req` type at dispatch time."
)]

use core::any::Any;
use std::collections::{BTreeMap, BTreeSet};

use aqc_file_engine_core::{EngineRequirement, MergedAssertion};

/// Declarative requirement for the `clippy.toml` engine.
#[derive(Debug, Clone, Default)]
pub struct ClippyTomlRequirement {
    pub msrv: Option<MergedAssertion<MsrvAssertion>>,
    pub thresholds: Option<MergedAssertion<ThresholdsAssertion>>,
    pub disallowed_methods: Option<MergedAssertion<BansAssertion>>,
    pub disallowed_types: Option<MergedAssertion<BansAssertion>>,
    pub disallowed_macros: Option<MergedAssertion<BansAssertion>>,
    #[expect(
        clippy::type_complexity,
        reason = "BTreeMap<setting, MergedAssertion<BoolAssertion>> is the natural keyed-by-setting shape."
    )]
    pub bools: BTreeMap<String, MergedAssertion<BoolAssertion>>,
    #[expect(
        clippy::type_complexity,
        reason = "BTreeMap<setting, MergedAssertion<StringAssertion>> is the natural keyed-by-setting shape."
    )]
    pub enums: BTreeMap<String, MergedAssertion<StringAssertion>>,
}

/// What must hold about `msrv`.
#[derive(Debug, Clone)]
pub enum MsrvAssertion {
    Equals(String),
    AtLeast(String),
    OneOf(BTreeSet<String>),
    Present,
    Absent,
}

/// What must hold about clippy's numeric threshold keys
/// (e.g. `cognitive-complexity-threshold`).
#[derive(Debug, Clone)]
pub enum ThresholdsAssertion {
    Equals(BTreeMap<String, u64>),
    AtMost(BTreeMap<String, u64>),
    AtLeast(BTreeMap<String, u64>),
    Present(BTreeSet<String>),
    Absent(BTreeSet<String>),
}

/// What must hold about a ban table (`disallowed-methods`, `disallowed-types`,
/// `disallowed-macros`). Same shape for all three; reused across targets.
#[derive(Debug, Clone)]
pub enum BansAssertion {
    Contains(Vec<BanEntry>),
    Excludes(BTreeSet<String>),
    IsExactly(Vec<BanEntry>),
}

/// One entry in a clippy ban list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BanEntry {
    pub path: String,
    pub reason: Option<String>,
}

/// What must hold about a boolean clippy setting.
#[derive(Debug, Clone)]
pub enum BoolAssertion {
    Equals(bool),
    Present,
    Absent,
}

/// What must hold about a string-valued (enum-style) clippy setting.
#[derive(Debug, Clone)]
pub enum StringAssertion {
    Equals(String),
    OneOf(BTreeSet<String>),
    Present,
    Absent,
}

impl EngineRequirement for ClippyTomlRequirement {
    fn engine_id(&self) -> &'static str {
        "aqc-clippy-toml-engine"
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
