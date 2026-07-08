//! Text file requirement model.

#![expect(
    clippy::module_name_repetitions,
    reason = "Public text-engine values use the TextFile prefix."
)]

use core::cmp::Ordering;
use core::fmt;
use std::path::{Component, Path, PathBuf};

use aqc_file_engine_core::{
    ConflictEntry, FileItemRequirement, ItemAssertionInput, ItemRequirements,
    RequiredItemResolution, ResolvedItemRequirements, ResolvedRequirement, ScalarAssertion,
    ScalarValue,
};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct TextFilePath(PathBuf);

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct TextSnippetId(String);

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct TextFileContents(Vec<u8>);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TextFileValueError {
    Empty { field: &'static str },
    AbsolutePath { value: PathBuf },
    ParentDirectory { value: PathBuf },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextSnippet {
    pub id: TextSnippetId,
    pub contents: TextFileContents,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextFileRequirement {
    pub path: TextFilePath,
    pub exact_contents: Option<ScalarAssertion<TextFileContents>>,
    pub required_snippets: ItemRequirements<TextSnippet>,
    pub executable: Option<ScalarAssertion<bool>>,
}

#[derive(Debug, Clone, Default)]
pub struct TextFileRequirements {
    pub files: ItemRequirements<TextFileRequirement>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedTextFileRequirement {
    pub exact_contents: Option<
        ResolvedRequirement<ScalarAssertion<TextFileContents>, ScalarAssertion<TextFileContents>>,
    >,
    pub required_snippets: ResolvedItemRequirements<TextSnippet>,
    pub executable: Option<ResolvedRequirement<ScalarAssertion<bool>, ScalarAssertion<bool>>>,
}

#[derive(Debug, Clone, Default)]
pub struct ResolvedTextFileRequirements {
    pub files: ResolvedItemRequirements<TextFileRequirement>,
}

impl TextFilePath {
    pub fn new(value: impl Into<PathBuf>) -> Result<Self, TextFileValueError> {
        let value = value.into();
        validate_relative_path(&value, "path")?;
        Ok(Self(value))
    }

    #[must_use]
    pub fn as_path(&self) -> &Path {
        &self.0
    }
}

impl TextSnippetId {
    pub fn new(value: impl Into<String>) -> Result<Self, TextFileValueError> {
        let value = value.into();
        if value.is_empty() {
            return Err(TextFileValueError::Empty { field: "snippet" });
        }
        Ok(Self(value))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TextFileContents {
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
            Self::AbsolutePath { value } => {
                write!(f, "path must be relative: {}", value.display())
            }
            Self::ParentDirectory { value } => {
                write!(
                    f,
                    "path must not contain parent directory: {}",
                    value.display()
                )
            }
        }
    }
}

impl fmt::Display for TextFilePath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0.display())
    }
}

impl fmt::Display for TextSnippetId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
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

impl FileItemRequirement for TextSnippet {
    type Identity = TextSnippetId;

    fn merge_identity(&self) -> Self::Identity {
        self.id.clone()
    }

    fn compose_item(
        key: &str,
        items: Vec<ItemAssertionInput<Self>>,
        conflicts: &mut Vec<ConflictEntry>,
    ) -> Option<RequiredItemResolution<Self>> {
        aqc_file_engine_core::compose_item_by(key, items, |item| item.contents.clone(), conflicts)
    }
}

impl FileItemRequirement for TextFileRequirement {
    type Identity = TextFilePath;

    fn merge_identity(&self) -> Self::Identity {
        self.path.clone()
    }

    fn compose_item(
        key: &str,
        items: Vec<ItemAssertionInput<Self>>,
        conflicts: &mut Vec<ConflictEntry>,
    ) -> Option<RequiredItemResolution<Self>> {
        crate::requirement::merge::compose_text_file(key, items, conflicts)
    }
}

fn validate_relative_path(value: &Path, field: &'static str) -> Result<(), TextFileValueError> {
    if value.as_os_str().is_empty() {
        return Err(TextFileValueError::Empty { field });
    }
    if value.is_absolute() {
        return Err(TextFileValueError::AbsolutePath {
            value: value.to_path_buf(),
        });
    }
    if value
        .components()
        .any(|component| matches!(component, Component::ParentDir))
    {
        return Err(TextFileValueError::ParentDirectory {
            value: value.to_path_buf(),
        });
    }
    Ok(())
}
