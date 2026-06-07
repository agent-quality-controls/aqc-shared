//! The porcelain v1 `-z` parser (pure; testable on fixture strings).

use crate::error::GitError;
use crate::status::{ChangeStatus, ColumnChange, WorktreeChange};

/// Parse `git status --porcelain=v1 -z` output.
///
/// Records are NUL-terminated `XY <path>`; a rename record is followed by one
/// extra NUL-terminated field: the source path.
///
/// # Errors
///
/// [`GitError::ParseError`] for records shorter than `XY ` or unknown codes.
pub fn parse_porcelain_v1z(text: &str) -> Result<Vec<WorktreeChange>, GitError> {
    let mut fields = text.split('\0').filter(|f| !f.is_empty());
    let mut changes = Vec::new();
    while let Some(record) = fields.next() {
        if record.len() < 4 {
            return Err(GitError::ParseError {
                message: format!("record too short: {record:?}"),
            });
        }
        let (code, path) = record.split_at(3);
        let mut code_chars = code.chars();
        let (index, worktree) = (
            code_chars.next().unwrap_or(' '),
            code_chars.next().unwrap_or(' '),
        );
        let status = classify(index, worktree).ok_or_else(|| GitError::ParseError {
            message: format!("unknown status code: {code:?}"),
        })?;
        let old_path = if has_source_field(status) {
            Some(
                fields
                    .next()
                    .ok_or_else(|| GitError::ParseError {
                        message: format!("rename record missing source path: {record:?}"),
                    })?
                    .replace('\\', "/"),
            )
        } else {
            None
        };
        changes.push(WorktreeChange {
            path: path.replace('\\', "/"),
            status,
            old_path,
        });
    }
    Ok(changes)
}

/// Map porcelain v1 `XY` columns to a [`ChangeStatus`]: whole-record states
/// first (`??`, `!!`, unmerged), then each column independently.
const fn classify(index: char, worktree: char) -> Option<ChangeStatus> {
    match (index, worktree) {
        ('?', '?') => Some(ChangeStatus::Untracked),
        ('!', '!') => Some(ChangeStatus::Ignored),
        ('U', _) | (_, 'U') | ('A', 'A') | ('D', 'D') => Some(ChangeStatus::Conflicted),
        (x, y) => match (column(x), column(y)) {
            // both empty would be a clean path; porcelain never emits it.
            (Ok(None), Ok(None)) | (Err(()), _) | (_, Err(())) => None,
            (Ok(idx), Ok(wt)) => Some(ChangeStatus::Tracked {
                index: idx,
                worktree: wt,
            }),
        },
    }
}

/// Parse one porcelain column character.
const fn column(code: char) -> Result<Option<ColumnChange>, ()> {
    match code {
        ' ' => Ok(None),
        'A' => Ok(Some(ColumnChange::Added)),
        'M' => Ok(Some(ColumnChange::Modified)),
        'D' => Ok(Some(ColumnChange::Deleted)),
        'R' => Ok(Some(ColumnChange::Renamed)),
        'C' => Ok(Some(ColumnChange::Copied)),
        'T' => Ok(Some(ColumnChange::TypeChanged)),
        _ => Err(()),
    }
}

/// True when the record is followed by a `-z` source-path field (renames and
/// copies, either column).
const fn has_source_field(status: ChangeStatus) -> bool {
    match status {
        ChangeStatus::Tracked { index, worktree } => {
            matches!(index, Some(ColumnChange::Renamed | ColumnChange::Copied))
                || matches!(worktree, Some(ColumnChange::Renamed | ColumnChange::Copied))
        }
        ChangeStatus::Conflicted | ChangeStatus::Untracked | ChangeStatus::Ignored => false,
    }
}
