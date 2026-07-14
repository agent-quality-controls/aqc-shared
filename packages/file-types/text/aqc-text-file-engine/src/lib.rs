//! Reusable text byte-stream mechanics for exact and contained contents.

#[cfg(feature = "api")]
mod engine;
#[cfg(feature = "api")]
mod reconcile;
#[cfg(feature = "api")]
mod requirement;

#[cfg(feature = "api")]
pub const ENGINE_ID: &str = "aqc-text-file-engine";
#[cfg(feature = "api")]
pub use aqc_file_engine_core::{Finding, ItemRequirements, Provenance, ScalarAssertion};
#[cfg(feature = "api")]
pub use engine::TextFileEngine;
#[cfg(feature = "api")]
pub use reconcile::reconcile_text_file;
#[cfg(feature = "api")]
pub use requirement::{
    ResolvedTextFileRequirements, TextFileContents, TextFileRequirements, TextFileValueError,
};
