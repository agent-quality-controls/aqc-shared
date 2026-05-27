//! Structured findings emitted by engines and linter adapters.
//!
//! One enum, every layer speaks it. Engines emit `Mismatch` /
//! `UnwritableRequiredKey` / `SchemaError` / `ParseError`. Linter
//! adapters emit `PolicyConflict`. Anything else catastrophic that's
//! still recoverable becomes `InternalError`.

use crate::types::{Provenance, Severity};

/// A structured finding emitted by a `FileEngine` or a linter adapter.
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
    /// The file isn't valid in its native grammar (e.g. malformed TOML).
    ParseError { message: String, severity: Severity },
    /// Two or more policies emitted irreconcilable contributions for the
    /// same target. The adapter aborts the merge for that field.
    PolicyConflict {
        target: String,
        contributors: Vec<Provenance>,
        detail: String,
        severity: Severity,
    },
    /// Engine- or adapter-internal failure (panic-class, but caught
    /// before it actually panics). Always `Severity::Error`.
    InternalError { message: String },
}
