//! Framework data types: `Provenance`, `MergedAssertion`, `EngineOutput`, and `Severity`.

use core::cmp::Ordering;

use crate::finding::Finding;
use crate::merge::ScalarValue;
use crate::toml_helpers::parse_version_tuple;

/// Identifies which policy contributed a requirement.
///
/// Carried through merge so findings can name the policies a user
/// would have to disable to drop the requirement.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Provenance {
    pub policy: String,
}

/// Severity of a `Finding`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Severity {
    Error,
    Warning,
    Info,
}

/// Domain scalar for config values, shared by every engine.
///
/// Value primitives are framework vocabulary (like [`Severity`]), defined
/// once; the per-engine "identical shapes are defined separately" rule governs
/// assertion enums, not primitives. No `Float`: no controllable key needs one,
/// and float equality is a comparison hazard.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum ConfigScalar {
    Str(String),
    Int(i64),
    Bool(bool),
}

/// Dotted numeric version value for fields that explicitly choose
/// `(major, minor, patch)` ordering.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DottedVersion(String);

impl DottedVersion {
    #[must_use]
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<String> for DottedVersion {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

impl From<&str> for DottedVersion {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl PartialOrd for DottedVersion {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for DottedVersion {
    fn cmp(&self, other: &Self) -> Ordering {
        parse_version_tuple(&self.0).cmp(&parse_version_tuple(&other.0))
    }
}

impl ScalarValue for DottedVersion {
    fn render(&self) -> String {
        self.0.clone()
    }

    fn compare_for_order(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl ScalarValue for ConfigScalar {
    fn render(&self) -> String {
        match self {
            Self::Str(value) => value.clone(),
            Self::Int(value) => value.to_string(),
            Self::Bool(value) => value.to_string(),
        }
    }

    fn compare_for_order(&self, _other: &Self) -> Option<Ordering> {
        None
    }
}

impl ScalarValue for String {
    fn render(&self) -> String {
        self.clone()
    }

    fn compare_for_order(&self, _other: &Self) -> Option<Ordering> {
        None
    }
}

impl ScalarValue for bool {
    fn render(&self) -> String {
        self.to_string()
    }

    fn compare_for_order(&self, _other: &Self) -> Option<Ordering> {
        None
    }
}

impl ScalarValue for i64 {
    fn render(&self) -> String {
        self.to_string()
    }

    fn compare_for_order(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl ScalarValue for u64 {
    fn render(&self) -> String {
        self.to_string()
    }

    fn compare_for_order(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// Policy-authored explanation carried by every assertion.
///
/// Surfaced on `Finding::Mismatch.message`. NEVER part of merge agreement:
/// two policies asserting the same value with different messages agree
/// (first message wins).
/// What an assertion does when the file does not exist yet.
///
/// `Writes`: exactly one correct value exists, so `init` writes it.
/// `ChecksOnly`: no single right answer exists (e.g. "one of these", "set to
/// anything"), so the engine can only check; from empty it reports an Error
/// and writes nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OnEmpty {
    Writes,
    ChecksOnly,
}

/// The classification every assertion type must answer in code.
///
/// Exhaustive matching makes a new variant uncompilable until its author
/// answers what happens on an empty file. Value-dependent cases compute the
/// answer (a dependency `Contains` writes only when the spec names a source).
pub trait OnEmptyClass {
    /// The class of this assertion value on a missing file.
    fn on_empty(&self) -> OnEmpty;
}

/// Result of a single reconcile operation against one file.
///
/// `expected_bytes` is the bytes `init` would write. `findings`
/// describe disagreements with disk (for `validate`) or other issues.
/// `init` refuses to write when any finding has `Severity::Error`.
#[derive(Debug, Clone)]
pub struct EngineOutput {
    pub expected_bytes: Vec<u8>,
    pub findings: Vec<Finding>,
}
