//! Change vocabulary: status matrix, one change, output filtering.

/// Porcelain version this crate speaks.
pub const PORCELAIN_VERSION: &str = "v1";

/// How one path differs from HEAD/index, per porcelain v1 columns.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChangeStatus {
    /// `A` in the index column.
    StagedNew,
    /// `M` in the index column.
    StagedModified,
    /// `D` in the index column.
    StagedDeleted,
    /// `R` in the index column.
    StagedRenamed,
    /// `M` in the worktree column.
    UnstagedModified,
    /// `D` in the worktree column.
    UnstagedDeleted,
    /// `R` in the worktree column.
    UnstagedRenamed,
    /// `??`.
    Untracked,
    /// `!!` (listed only with `include_ignored`).
    Ignored,
}

/// One changed path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorktreeChange {
    /// Repo-relative, `/` separators, UTF-8.
    pub path: String,
    /// How it changed.
    pub status: ChangeStatus,
    /// Rename source path, when `status` is a rename.
    pub old_path: Option<String>,
}

/// Output filtering.
#[derive(Debug, Clone, Copy)]
pub struct PorcelainOptions {
    /// Keep `!!` entries.
    pub include_ignored: bool,
    /// Keep `??` entries.
    pub include_untracked: bool,
}

impl Default for PorcelainOptions {
    fn default() -> Self {
        Self {
            include_ignored: false,
            include_untracked: true,
        }
    }
}
