//! Pure queries over worktree changes.

use std::path::Path;

use crate::error::GitError;
use crate::git::worktree_changes;
use crate::status::{PorcelainOptions, WorktreeChange};

/// True when [`worktree_changes`] is empty (respecting `options`).
///
/// # Errors
///
/// Same as [`worktree_changes`].
pub fn is_worktree_clean(
    repo_root: impl AsRef<Path>,
    options: PorcelainOptions,
) -> Result<bool, GitError> {
    Ok(worktree_changes(repo_root, options)?.is_empty())
}

/// The subset of `changes` whose `path` (or `old_path` for renames) equals an
/// entry in `paths` or lies under one (`"<entry>/"` prefix — directory
/// boundary, never substring).
#[must_use]
pub fn changes_affecting_paths(changes: &[WorktreeChange], paths: &[&str]) -> Vec<WorktreeChange> {
    let hits = |candidate: &str| {
        paths
            .iter()
            .any(|p| candidate == *p || candidate.starts_with(&format!("{p}/")))
    };
    changes
        .iter()
        .filter(|c| hits(&c.path) || c.old_path.as_deref().is_some_and(hits))
        .cloned()
        .collect()
}

/// Convenience: [`worktree_changes`] then [`changes_affecting_paths`].
///
/// # Errors
///
/// Same as [`worktree_changes`].
pub fn dirty_paths(
    repo_root: impl AsRef<Path>,
    paths: &[&str],
    options: PorcelainOptions,
) -> Result<Vec<WorktreeChange>, GitError> {
    let changes = worktree_changes(repo_root, options)?;
    Ok(changes_affecting_paths(&changes, paths))
}
