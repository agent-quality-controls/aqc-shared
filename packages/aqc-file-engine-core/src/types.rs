//! Framework data types: `Provenance`, `MergedAssertion`, `Finding`,
//! `EngineOutput`, `EngineError`, `MergeConflict`, `Severity`.

/// Identifier for a Guardrail3 policy. Always a `String`.
pub type PolicyId = String;

/// Identifies which policy contributed a requirement.
///
/// Carried through merge so findings can name the policies a user
/// would have to disable to drop the requirement.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Provenance {
    pub policy: PolicyId,
}

/// Per-policy contributions for one assertion target, kept as slices.
///
/// The linter adapter does *not* squash contributions into a single
/// merged assertion. It preserves each policy's contribution alongside
/// its `Provenance`. Per-element attribution is derived at use time
/// (engine-side) by walking `contributions`.
#[derive(Debug, Clone)]
#[expect(
    clippy::type_complexity,
    reason = "Vec<(Provenance, A)> is the natural shape: an ordered list of attributed contributions. A named type alias hides the parameterization without value."
)]
pub struct MergedAssertion<A> {
    pub contributions: Vec<(Provenance, A)>,
}

/// Severity of a `Finding`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Severity {
    Error,
    Warning,
    Info,
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

/// A structured finding emitted by a `FileEngine`.
#[derive(Debug, Clone)]
pub enum Finding {
    /// A key on disk disagrees with what the requirement asserts.
    Mismatch {
        path: String,
        current: Option<String>,
        expected: String,
        severity: Severity,
        attribution: Vec<Provenance>,
    },
    /// Reconcile knew where to write but the file role forbids it.
    UnwritableRequiredKey {
        path: String,
        expected: String,
        attribution: Vec<Provenance>,
    },
    /// The file violates its own schema, independent of our requirements.
    SchemaError {
        path: String,
        message: String,
        severity: Severity,
    },
}

/// Engine-internal failure that isn't a `Finding` (e.g. parser crashed).
#[derive(Debug)]
pub enum EngineError {
    Parse(String),
    Other(String),
}

impl std::fmt::Display for EngineError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Parse(msg) => write!(f, "parse error: {msg}"),
            Self::Other(msg) => write!(f, "engine error: {msg}"),
        }
    }
}

impl std::error::Error for EngineError {}

/// Emitted by the linter-adapter merge step when contributions for a
/// single target are incompatible.
#[derive(Debug, Clone)]
pub struct MergeConflict {
    pub target: String,
    pub contributors: Vec<Provenance>,
    pub detail: String,
}

impl std::fmt::Display for MergeConflict {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "merge conflict on {}: {} (contributors: {:?})",
            self.target, self.detail, self.contributors
        )
    }
}

impl std::error::Error for MergeConflict {}
