//! Text file requirement model and merge logic.

mod merge;
mod model;

pub use model::{
    ResolvedTextFileRequirements, TextFileContents, TextFileRequirements, TextFileValueError,
    TextSnippet, TextSnippetId,
};
