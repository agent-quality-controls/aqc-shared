//! Git hooks file engine.

#[cfg(feature = "api")]
mod engine;
#[cfg(feature = "api")]
pub mod requirement;

#[cfg(feature = "api")]
pub use aqc_file_engine_core::{
    Engine, EngineFileOutput, EngineFileState, EngineOutput, EngineRequirement, Finding,
    ItemRequirements, Provenance, ScalarAssertion,
};
#[cfg(feature = "api")]
pub use aqc_text_engine_core::{
    TextFileContents, TextFilePath, TextFileRequirement, TextFileRequirements, TextSnippet,
    TextSnippetId,
};
#[cfg(feature = "api")]
pub use engine::GitHooksEngine;
#[cfg(feature = "api")]
pub use requirement::{GitHooksRequirements, ResolvedGitHooksRequirements};

#[cfg(feature = "api")]
pub const ENGINE_ID: &str = "git-hooks";
