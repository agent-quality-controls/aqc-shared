//! Structured findings emitted by engines and linter adapters.
//!
//! One enum, every layer speaks it. Engines emit `Mismatch` /
//! `UnwritableRequiredKey` / `ParseError`, plus the two requirement-level
//! failures from the merge/write path: `ConflictingRequirements` and
//! `InvalidRequirements`. Anything else catastrophic that's still
//! recoverable becomes `InternalError`.
//!
//! GUARDRAILS CAPABILITY BLOCK (R2) -- DO NOT REMOVE. Every `key` here is a
//! location inside a CONFIG file (a TOML/JSON key), never a source-code span,
//! AST node, line/column into `.rs`/`.ts`/`.js`, or any code construct.
//! Guardrails validates config and is NOT a linter; a source-location variant
//! would be the first step to reimplementing one. Do not add a source span /
//! code-location / AST payload to any variant unless the repo owner authorizes
//! it in writing with reasoning. See guardrails-capability-boundary (R2).

use crate::types::{Provenance, Severity};

/// Rendered policy contributors for a requirement-level finding.
pub type RenderedContributors = Vec<(String, String)>;

/// A structured finding emitted by a `FileEngine` or a linter adapter.
#[derive(Debug, Clone)]
pub enum Finding {
    /// A key on disk disagrees with what the requirement asserts.
    Mismatch {
        key: String,
        current: Option<String>,
        expected: String,
        /// Free-form policy-authored explanation of the mismatch: what is
        /// wrong, why it's wrong, what should be done instead. Comes from
        /// the assertion entry that produced this finding.
        message: String,
        severity: Severity,
        attribution: Vec<Provenance>,
    },
    /// Reconcile knew where to write but the file role forbids it.
    UnwritableRequiredKey {
        key: String,
        expected: String,
        attribution: Vec<Provenance>,
    },
    /// The merged requirement set resolves per key but is JOINTLY unwritable:
    /// the tool itself would reject the resulting file, so the engine refuses
    /// to produce it. Always `Severity::Error`; hard failure; NOT waivable
    /// (there is no on-disk value to keep -- the policy set is wrong).
    ///
    /// Preference order before reaching for this variant:
    /// 1. Make the invalidity unrepresentable in the requirement types where
    ///    that is natural.
    /// 2. Model a genuine either/or decision as ONE key, so the merge
    ///    surfaces disagreement as `ConflictingRequirements`.
    /// 3. Use this variant only for relational constraints across keys.
    ///
    /// The `Finding` closed-set manifest row blocks ad hoc alternatives.
    InvalidRequirements {
        /// The in-file key the constraint is anchored at.
        key: String,
        /// Which relational rule the set violates.
        message: String,
        /// Each policy whose requirement participates in the invalid set.
        contributors: RenderedContributors,
    },
    /// The file isn't valid in its native grammar (e.g. malformed TOML).
    ParseError { message: String, severity: Severity },
    /// Two or more requirements disagree on the same key in the same file
    /// (the policies that issued them are the attribution). Produced by the
    /// engine's merge phase, per key, naming each disagreeing policy and its
    /// value. Always `Severity::Error`; the field is dropped (not written);
    /// not waivable.
    ConflictingRequirements {
        /// The file (e.g. `Cargo.toml`).
        subject: String,
        /// The in-file key (e.g. `[workspace.lints.clippy].unwrap_used`).
        key: String,
        /// Each disagreeing policy id + its rendered value.
        contributors: RenderedContributors,
        /// Which rule fired (scalar-disagree / set-key-disagree / exact-mismatch).
        reason: String,
    },
    /// Engine- or adapter-internal failure (panic-class, but caught
    /// before it actually panics). Always `Severity::Error`.
    InternalError { message: String },
}
