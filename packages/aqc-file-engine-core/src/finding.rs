//! Structured findings emitted by engines and linter adapters.
//!
//! One enum, every layer speaks it. Engines emit `Mismatch` /
//! `UnwritableRequiredKey` / `SchemaError` / `ParseError`. Linter
//! adapters emit `PolicyConflict`. Anything else catastrophic that's
//! still recoverable becomes `InternalError`.
//!
//! GUARDRAILS CAPABILITY BLOCK (R2) -- DO NOT REMOVE. Every `path` here is a
//! location inside a CONFIG file (a TOML/JSON key), never a source-code span,
//! AST node, line/column into `.rs`/`.ts`/`.js`, or any code construct.
//! Guardrails validates config and is NOT a linter; a source-location variant
//! would be the first step to reimplementing one. Do not add a source span /
//! code-location / AST payload to any variant unless the repo owner authorizes
//! it in writing with reasoning. See guardrails-capability-boundary (R2).

use crate::types::{PolicyId, Provenance, Severity};

/// Each disagreeing policy id paired with its value, rendered for display.
pub type ConflictContributors = Vec<(PolicyId, String)>;

/// A structured finding emitted by a `FileEngine` or a linter adapter.
#[derive(Debug, Clone)]
pub enum Finding {
    /// A key on disk disagrees with what the requirement asserts.
    Mismatch {
        path: String,
        current: Option<String>,
        expected: String,
        /// Free-form policy-authored explanation of the mismatch: what is
        /// wrong, why it's wrong, what should be done instead. Sourced from
        /// the assertion entry that produced this finding.
        message: String,
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
    /// The file isn't valid in its native grammar (e.g. malformed TOML).
    ParseError { message: String, severity: Severity },
    /// Two or more enabled policies require different values for the same key
    /// in the same file. Produced by the engine's merge phase, per key, naming
    /// each disagreeing policy and its value. Always `Severity::Error`; the field
    /// is dropped (not written); not waivable.
    PolicyConflict {
        /// The file (e.g. `Cargo.toml`).
        subject: String,
        /// The in-file key (e.g. `[workspace.lints.clippy].unwrap_used`).
        key: String,
        /// Each disagreeing policy id + its rendered value.
        contributors: ConflictContributors,
        /// Which rule fired (scalar-disagree / set-key-disagree / exact-mismatch).
        reason: String,
    },
    /// Engine- or adapter-internal failure (panic-class, but caught
    /// before it actually panics). Always `Severity::Error`.
    InternalError { message: String },
}
