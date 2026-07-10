//! Text byte-stream requirement model.

#![allow(
    clippy::module_name_repetitions,
    reason = "Public text-engine values use the TextFile prefix."
)]

use core::any::Any;
use core::cmp::Ordering;
use core::fmt;

use aqc_file_engine_core::{
    ConflictEntry, EngineRequirement, FileItemRequirement, ItemAssertionInput, ItemRequirements,
    RequiredItemResolution, ResolvedItemRequirements, ResolvedRequirement, ScalarAssertion,
    ScalarValue,
};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
/// Expected byte contents for a whole text file or contained item.
pub struct TextFileContents(Vec<u8>);

#[derive(Debug, Clone, PartialEq, Eq)]
/// Validation errors for text requirement values.
pub enum TextFileValueError {
    /// A required text value was empty.
    Empty { field: &'static str },
}

#[derive(Debug, Clone, Default)]
/// Requirements for a generic text byte stream.
pub struct TextFileRequirements {
    /// Optional exact file contents assertion.
    pub exact_contents: Option<ScalarAssertion<TextFileContents>>,
    /// Byte sequences that must appear when exact file contents are not required.
    pub contents: ItemRequirements<TextFileContents>,
}

#[derive(Debug, Clone, Default)]
/// Resolved requirements for a generic text byte stream.
pub struct ResolvedTextFileRequirements {
    /// Resolved exact file contents assertion.
    pub exact_contents: Option<
        ResolvedRequirement<ScalarAssertion<TextFileContents>, ScalarAssertion<TextFileContents>>,
    >,
    /// Resolved contained byte assertions.
    pub contents: ResolvedItemRequirements<TextFileContents>,
}

impl EngineRequirement for TextFileRequirements {
    fn engine_id(&self) -> &'static str {
        crate::ENGINE_ID
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl TextFileContents {
    /// Build non-empty text file contents.
    ///
    /// # Errors
    ///
    /// Returns [`TextFileValueError::Empty`] when `value` is empty.
    pub fn new(value: impl Into<Vec<u8>>) -> Result<Self, TextFileValueError> {
        let value = value.into();
        if value.is_empty() {
            return Err(TextFileValueError::Empty { field: "contents" });
        }
        Ok(Self(value))
    }

    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}

impl fmt::Display for TextFileValueError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty { field } => write!(f, "{field} must not be empty"),
        }
    }
}

impl fmt::Display for TextFileContents {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("contents")
    }
}

impl ScalarValue for TextFileContents {
    fn render(&self) -> String {
        format!("{} bytes", self.0.len())
    }

    fn compare_for_order(&self, _other: &Self) -> Option<Ordering> {
        None
    }
}

impl FileItemRequirement for TextFileContents {
    type Identity = Self;

    fn merge_identity(&self) -> Self::Identity {
        self.clone()
    }

    fn compose_item(
        key: &str,
        items: Vec<ItemAssertionInput<Self>>,
        conflicts: &mut Vec<ConflictEntry>,
    ) -> Option<RequiredItemResolution<Self>> {
        aqc_file_engine_core::compose_item_by(key, items, Clone::clone, conflicts)
    }
}
