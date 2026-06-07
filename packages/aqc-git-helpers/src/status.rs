//! Change vocabulary: status matrix, one change, output filtering.

/// Porcelain version this crate speaks.
pub const PORCELAIN_VERSION: &str = "v1";

/// One porcelain column's change kind (`X` index column / `Y` worktree column).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColumnChange {
    /// `A`.
    Added,
    /// `M`.
    Modified,
    /// `D`.
    Deleted,
    /// `R`.
    Renamed,
    /// `C`.
    Copied,
    /// `T`.
    TypeChanged,
}

/// How one path differs from HEAD/index.
///
/// Porcelain `XY` is a MATRIX: a path can be staged-modified AND
/// unstaged-modified at once (`MM`), so tracked state carries both columns
/// instead of collapsing them into one variant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChangeStatus {
    /// A tracked path; at least one column is set.
    Tracked {
        /// The index (staged) column, `X`.
        index: Option<ColumnChange>,
        /// The worktree (unstaged) column, `Y`.
        worktree: Option<ColumnChange>,
    },
    /// An unmerged path (`U` in either column, `AA`, `DD`). Dirty, fail-safe.
    Conflicted,
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
