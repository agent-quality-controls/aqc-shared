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

/// What must hold about `msrv`. Each value-carrying variant carries a
/// `(value, message)` pair; `Present`/`Absent` carry the message.
#[derive(Debug, Clone)]
pub enum MsrvAssertion {
    /// `(version, message)`.
    Equals(String, String),
    /// `(version, message)`.
    AtLeast(String, String),
    /// `(allowed versions, message)`.
    OneOf(BTreeSet<String>, String),
    /// `(message)` -- msrv must be set.
    Present(String),
    /// `(message)` -- msrv must not be set.
    Absent(String),
}

/// What must hold about clippy's numeric threshold keys
/// (e.g. `cognitive-complexity-threshold`). Map values are
/// `(threshold, message)` pairs; set-shaped variants map name -> message.
#[expect(
    clippy::type_complexity,
    reason = "BTreeMap<String, (value, message)> is the explicit per-entry shape; aliasing the inner tuple obscures the (value, message) pattern used uniformly across assertion types."
)]
#[derive(Debug, Clone)]
pub enum ThresholdsAssertion {
    Equals(BTreeMap<String, (u64, String)>),
    AtMost(BTreeMap<String, (u64, String)>),
    AtLeast(BTreeMap<String, (u64, String)>),
    /// name -> message (key must be present).
    Present(BTreeMap<String, String>),
    /// name -> message (key must be absent).
    Absent(BTreeMap<String, String>),
}

/// What must hold about a ban table (`disallowed-methods`, `disallowed-types`,
/// `disallowed-macros`). Same shape for all three; reused across targets.
#[derive(Debug, Clone)]
pub enum BansAssertion {
    Contains(Vec<BanEntry>),
    /// path -> message (must not be banned).
    Excludes(BTreeMap<String, String>),
    IsExactly(Vec<BanEntry>),
}

/// One entry in a clippy ban list. `message` is policy-mandatory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BanEntry {
    pub path: String,
    pub message: String,
}

/// What must hold about a boolean clippy setting.
#[derive(Debug, Clone)]
pub enum BoolAssertion {
    /// `(value, message)`.
    Equals(bool, String),
    /// `(message)`.
    Present(String),
    /// `(message)`.
    Absent(String),
}

/// What must hold about a string-valued (enum-style) clippy setting.
#[derive(Debug, Clone)]
pub enum StringAssertion {
    /// `(value, message)`.
    Equals(String, String),
    /// `(allowed values, message)`.
    OneOf(BTreeSet<String>, String),
    /// `(message)`.
    Present(String),
    /// `(message)`.
    Absent(String),
}

impl EngineRequirement for ClippyTomlRequirement {
    fn engine_id(&self) -> &'static str {
        crate::ENGINE_ID
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
