//! Reusable text file mechanics for exact contents, snippets, and executable bits.

#[cfg(feature = "api")]
mod reconcile;
#[cfg(feature = "api")]
mod requirement;

#[cfg(feature = "api")]
pub use aqc_file_engine_core::{
    EngineFileOutput, EngineFileState, Finding, ItemRequirements, Provenance, ScalarAssertion,
};
#[cfg(feature = "api")]
pub use reconcile::reconcile_text_files;
#[cfg(feature = "api")]
pub use requirement::{
    ResolvedTextFileRequirement, ResolvedTextFileRequirements, TextFileContents, TextFilePath,
    TextFileRequirement, TextFileRequirements, TextFileValueError, TextSnippet, TextSnippetId,
};
