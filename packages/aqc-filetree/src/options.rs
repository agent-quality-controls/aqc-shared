//! Walk configuration: symlink policy, skip presets, recovery rules, errors.

use std::path::PathBuf;

use crate::tree::FileKind;

/// The only symlink control: traverse, record, or skip.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SymlinkPolicy {
    /// Default: symlinks neither traversed nor listed.
    #[default]
    Skip,
    /// Listed as [`FileKind::Symlink`], not traversed.
    Record,
    /// Traversed; targets appear as normal entries.
    Follow,
}

/// Skip-list presets; constants only, merged by the caller.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkipDirPreset {
    /// `.git`.
    Common,
    /// `target`.
    Rust,
    /// `node_modules`, `dist`.
    Node,
    /// `__pycache__`, `.venv`, `venv`, `.pytest_cache`, `.mypy_cache`, `.tox`.
    Python,
    /// `bin`, `obj`.
    DotNet,
}

impl SkipDirPreset {
    /// The directory name components this preset adds.
    #[must_use]
    pub const fn names(self) -> &'static [&'static str] {
        match self {
            Self::Common => &[".git"],
            Self::Rust => &["target"],
            Self::Node => &["node_modules", "dist"],
            Self::Python => &[
                "__pycache__",
                ".venv",
                "venv",
                ".pytest_cache",
                ".mypy_cache",
                ".tox",
            ],
            Self::DotNet => &["bin", "obj"],
        }
    }

    /// Merge presets into one deduplicated skip list.
    #[must_use]
    pub fn merge(presets: &[Self]) -> Vec<String> {
        let mut names: Vec<String> = presets
            .iter()
            .flat_map(|p| p.names().iter().map(|n| (*n).to_owned()))
            .collect();
        names.sort_unstable();
        names.dedup();
        names
    }
}

/// Phase 2 path predicates; OR across fields. Product-specific filename
/// lists live in callers, not in this crate.
#[derive(Debug, Clone, Default)]
pub struct RecoveryRules {
    /// Match the file base name exactly.
    pub exact_file_names: Vec<String>,
    /// Match a file base name prefix.
    pub file_name_prefixes: Vec<String>,
    /// Match a directory base name (presence sentinel).
    pub directory_names: Vec<String>,
    /// Match a full `rel_path` suffix.
    pub rel_path_suffixes: Vec<String>,
}

impl RecoveryRules {
    /// True when the (`rel_path`, base name, kind) matches any rule.
    pub(crate) fn matches(&self, rel_path: &str, name: &str, kind: FileKind) -> bool {
        match kind {
            FileKind::Directory => self.directory_names.iter().any(|d| d == name),
            FileKind::File | FileKind::Symlink => {
                self.exact_file_names.iter().any(|f| f == name)
                    || self.file_name_prefixes.iter().any(|p| name.starts_with(p))
                    || self
                        .rel_path_suffixes
                        .iter()
                        .any(|sfx| rel_path.ends_with(sfx))
            }
        }
    }
}

/// Walk configuration; defaults per `plan.md`.
#[derive(Debug, Clone)]
pub struct WalkOptions {
    /// Phase 1 honors `.gitignore` / `.ignore`.
    pub respect_gitignore: bool,
    /// Dotfiles are not skipped for being hidden.
    pub include_hidden: bool,
    /// Symlink handling (both phases).
    pub symlink_policy: SymlinkPolicy,
    /// Never descend into directories with these final name components
    /// (both phases; prunes descent by artifact-folder name, distinct from
    /// gitignore's VCS rules).
    pub skip_dir_names: Vec<String>,
    /// Never enter these root-relative subtrees (both phases).
    pub skip_path_prefixes: Vec<String>,
    /// `None` = unlimited; `Some(n)` = max directory depth below root.
    pub max_depth: Option<u32>,
    /// Phase 2 rules; `None` = phase 2 off.
    pub recovery: Option<RecoveryRules>,
    /// Case sensitivity for [`FileTree::glob`] when called through helpers.
    pub glob_case_sensitive: bool,
}

impl Default for WalkOptions {
    fn default() -> Self {
        Self {
            respect_gitignore: true,
            include_hidden: true,
            symlink_policy: SymlinkPolicy::Skip,
            skip_dir_names: SkipDirPreset::merge(&[
                SkipDirPreset::Common,
                SkipDirPreset::Rust,
                SkipDirPreset::Node,
            ]),
            skip_path_prefixes: Vec::new(),
            max_depth: None,
            recovery: None,
            glob_case_sensitive: true,
        }
    }
}

/// Why a walk failed.
#[derive(Debug)]
pub enum WalkError {
    /// The root path does not exist.
    RootNotFound,
    /// The root path exists but is not a directory.
    RootNotADirectory,
    /// An OS error during the walk.
    Io {
        /// Where the error occurred (the root or an entry).
        path: PathBuf,
        /// The underlying error.
        source: std::io::Error,
    },
}

impl core::fmt::Display for WalkError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::RootNotFound => write!(f, "walk root not found"),
            Self::RootNotADirectory => write!(f, "walk root is not a directory"),
            Self::Io { path, source } => write!(f, "io error on {}: {source}", path.display()),
        }
    }
}

impl core::error::Error for WalkError {}
