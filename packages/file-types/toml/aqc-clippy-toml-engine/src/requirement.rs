//! Declarative requirement and assertion types accepted by `ClippyTomlEngine`.

use std::collections::{BTreeMap, BTreeSet};

use aqc_file_engine_core::MergedAssertion;

/// Declarative requirement for the `clippy.toml` engine.
#[derive(Debug, Clone, Default)]
pub struct ClippyTomlRequirement {
    pub msrv: Option<MergedAssertion<MsrvAssertion>>,
    pub method_bans: Option<MergedAssertion<MethodBansAssertion>>,
    pub thresholds: Option<MergedAssertion<ThresholdsAssertion>>,
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

/// What must hold about the `disallowed-methods` array.
#[derive(Debug, Clone)]
pub enum MethodBansAssertion {
    Contains(Vec<MethodBanEntry>),
    Excludes(BTreeSet<String>),
    IsExactly(Vec<MethodBanEntry>),
}

/// One entry in `disallowed-methods`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MethodBanEntry {
    pub path: String,
    pub reason: String,
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
