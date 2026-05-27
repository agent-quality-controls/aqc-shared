//! Shared framework types for `aqc-{domain}-engine` crates.
//!
//! See the architecture plan:
//! `guardrail3/.plans/g3v2-architecture/2026-05-21-195830-repo-workspace-plugin-generation-model.md`.
//!
//! Every type in this crate is pure data; this crate performs zero I/O.

#[cfg(feature = "api")]
pub mod engine;
#[cfg(feature = "api")]
pub mod finding;
#[cfg(feature = "api")]
pub mod requirement;
#[cfg(feature = "api")]
pub mod toml_helpers;
#[cfg(feature = "api")]
pub mod types;

#[cfg(feature = "api")]
pub use engine::FileEngine;
#[cfg(feature = "api")]
pub use finding::Finding;
#[cfg(feature = "api")]
pub use requirement::EngineRequirement;
#[cfg(feature = "api")]
pub use toml_helpers::{parse_or_report, parse_version_tuple};
#[cfg(feature = "api")]
pub use types::{EngineOutput, MergedAssertion, PolicyId, Provenance, Severity};
