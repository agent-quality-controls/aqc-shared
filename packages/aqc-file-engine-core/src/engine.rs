//! `FileEngine` trait: the engine surface every `aqc-{domain}-engine` crate
//! implements.

use crate::types::{EngineError, EngineOutput};

/// A file engine: reconciles bytes-on-disk against typed declarative
/// requirements, returning both the bytes `init` would write and the
/// findings `validate` would report.
///
/// Engines are pure functions. They do not perform I/O.
#[expect(
    clippy::module_name_repetitions,
    reason = "FileEngine is the canonical trait name; renaming it loses the connection to the file-engines abstraction in plans and call sites."
)]
pub trait FileEngine<Req> {
    /// Apply `requirement` against `current_bytes`, returning what `init`
    /// would write and what `validate` would report.
    ///
    /// # Errors
    ///
    /// Returns `EngineError` when the engine can't process the input,
    /// e.g. the bytes don't parse as the file's grammar. Domain-level
    /// disagreements are surfaced as `Finding`s inside `EngineOutput`,
    /// not as errors.
    fn reconcile(
        current_bytes: Option<&[u8]>,
        requirement: &Req,
    ) -> Result<EngineOutput, EngineError>;
}
