//! Framework data types: `Provenance`, `MergedAssertion`, `EngineOutput`, `Severity`, `PolicyId`.

use crate::finding::Finding;

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
