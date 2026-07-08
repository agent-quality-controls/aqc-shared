//! Text file requirement model and merge logic.

mod merge;
mod model;

pub use model::{
    ResolvedTextFileRequirement, ResolvedTextFileRequirements, TextFileContents, TextFilePath,
    TextFileRequirement, TextFileRequirements, TextFileValueError, TextSnippet, TextSnippetId,
};
