//! `FileEngine` trait: the engine surface every `aqc-{domain}-engine` crate
//! implements.

use crate::types::EngineOutput;

/// A file engine: reconciles bytes-on-disk against typed declarative
/// requirements, returning both the bytes `init` would write and the
/// findings `validate` would report.
///
/// Engines are pure functions. They do not perform I/O. They never
/// return an error: catastrophic failures (parse failures, internal
/// invariant violations) surface as `Finding`s inside `EngineOutput`.
#[expect(
    clippy::module_name_repetitions,
    reason = "FileEngine is the canonical trait name; renaming it loses the connection to the file-engines abstraction in plans and call sites."
)]
pub trait FileEngine<Req> {
    /// Apply `requirement` against `current_bytes`, returning what `init`
    /// would write and what `validate` would report.
    fn reconcile(current_bytes: Option<&[u8]>, requirement: &Req) -> EngineOutput;
}
