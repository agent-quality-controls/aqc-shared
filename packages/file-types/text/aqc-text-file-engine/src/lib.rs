//! Reusable text file engine for exact contents, snippets, and executable bits.

#[cfg(feature = "api")]
mod engine;
#[cfg(feature = "api")]
mod reconcile;
#[cfg(feature = "api")]
mod requirement;

#[cfg(feature = "api")]
pub use aqc_file_engine_core::{
    Engine, EngineOutput, EngineRequirement, ItemRequirements, Provenance, ScalarAssertion,
};
#[cfg(feature = "api")]
pub use engine::TextFileEngine;
#[cfg(feature = "api")]
pub use requirement::{
    ResolvedTextFileRequirement, ResolvedTextFileRequirements, TextFileContents, TextFilePath,
    TextFileRequirement, TextFileRequirements, TextFileValueError, TextSnippet, TextSnippetId,
};

#[cfg(feature = "api")]
pub const ENGINE_ID: &str = "aqc-text-file-engine";
