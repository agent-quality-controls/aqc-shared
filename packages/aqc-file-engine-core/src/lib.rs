//! Shared framework types for `aqc-{domain}-engine` crates.
//!
//! See the architecture plan:
//! `guardrail3/.plans/g3v2-architecture/2026-05-21-195830-repo-workspace-plugin-generation-model.md`.
//!
//! Every type in this crate is pure data; this crate performs zero I/O.

#[cfg(feature = "api")]
pub mod engine;
#[cfg(feature = "api")]
pub mod types;

#[cfg(feature = "api")]
pub use engine::FileEngine;
#[cfg(feature = "api")]
pub use types::{
    EngineError, EngineOutput, Finding, MergeConflict, MergedAssertion, PolicyId, Provenance,
    Severity,
};
