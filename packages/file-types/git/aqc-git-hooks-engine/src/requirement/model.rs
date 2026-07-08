//! Git hooks requirement model.

use core::any::Any;
use core::fmt;
use std::path::{Component, Path, PathBuf};

use aqc_file_engine_core::EngineRequirement;
use aqc_text_engine_core::{ResolvedTextFileRequirements, TextFileRequirements};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct GitHooksPath(PathBuf);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GitHooksValueError {
    Empty,
    AbsolutePath { value: PathBuf },
    ParentDirectory { value: PathBuf },
}

#[derive(Debug, Clone, Default)]
pub struct GitHooksRequirements {
    pub files: TextFileRequirements,
}

#[derive(Debug, Clone, Default)]
pub struct ResolvedGitHooksRequirements {
    pub files: ResolvedTextFileRequirements,
}

impl GitHooksPath {
    pub fn new(value: impl Into<PathBuf>) -> Result<Self, GitHooksValueError> {
        let value = value.into();
        validate_relative_path(&value)?;
        Ok(Self(value))
    }

    #[must_use]
    pub fn dot_githooks() -> Self {
        Self(PathBuf::from(".githooks"))
    }

    #[must_use]
    pub fn as_path(&self) -> &Path {
        &self.0
    }
}

impl fmt::Display for GitHooksValueError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => f.write_str("path must not be empty"),
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

impl EngineRequirement for GitHooksRequirements {
    fn engine_id(&self) -> &'static str {
        crate::ENGINE_ID
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

fn validate_relative_path(value: &Path) -> Result<(), GitHooksValueError> {
    if value.as_os_str().is_empty() {
        return Err(GitHooksValueError::Empty);
    }
    if value.is_absolute() {
        return Err(GitHooksValueError::AbsolutePath {
            value: value.to_path_buf(),
        });
    }
    if value
        .components()
        .any(|component| matches!(component, Component::ParentDir))
    {
        return Err(GitHooksValueError::ParentDirectory {
            value: value.to_path_buf(),
        });
    }
    Ok(())
}
