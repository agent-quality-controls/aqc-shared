//! Framework data types: `Provenance`, `ConfigScalar`, `DottedVersion`, and `Severity`.

use core::cmp::Ordering;
use std::path::PathBuf;

use crate::finding::Finding;
use crate::merge::ScalarValue;
use crate::version::parse_version_tuple;
use serde::{Deserialize, Serialize};

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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
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

/// Current state for one file an erased engine reconciles.
#[derive(Debug, Clone)]
pub struct EngineFileState {
    pub path: PathBuf,
    pub bytes: Option<Vec<u8>>,
    pub executable: Option<bool>,
}

/// Result of reconciling one output file.
#[derive(Debug, Clone)]
pub struct EngineFileOutput {
    pub path: PathBuf,
    pub expected_bytes: Vec<u8>,
    pub expected_executable: Option<bool>,
    pub findings: Vec<Finding>,
}

/// Result of a reconcile operation.
///
/// `files` contains the file bytes and metadata `init` would write. `findings`
/// contains repo-level or engine-level findings not tied to one output file.
#[derive(Debug, Clone)]
pub struct EngineOutput {
    pub files: Vec<EngineFileOutput>,
    pub findings: Vec<Finding>,
}

impl EngineOutput {
    #[must_use]
    pub fn single(expected_bytes: Vec<u8>, findings: Vec<Finding>) -> Self {
        Self {
            files: vec![EngineFileOutput {
                path: PathBuf::new(),
                expected_bytes,
                expected_executable: None,
                findings: Vec::new(),
            }],
            findings,
        }
    }

    #[must_use]
    pub fn with_single_path(mut self, path: PathBuf) -> Self {
        for file in &mut self.files {
            if file.path.as_os_str().is_empty() {
                file.path = path.clone();
            }
        }
        self
    }
}
