//! The porcelain v1 `-z` parser (pure; testable on fixture strings).

use crate::error::GitError;
use crate::status::{ChangeStatus, WorktreeChange};

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
        let old_path = if matches!(
            status,
            ChangeStatus::StagedRenamed | ChangeStatus::UnstagedRenamed
        ) {
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

/// Map porcelain v1 `XY` columns to a [`ChangeStatus`]. Index column wins for
/// staged states; the worktree column classifies unstaged states.
const fn classify(index: char, worktree: char) -> Option<ChangeStatus> {
    match (index, worktree) {
        ('?', '?') => Some(ChangeStatus::Untracked),
        ('!', '!') => Some(ChangeStatus::Ignored),
        ('A', _) => Some(ChangeStatus::StagedNew),
        ('M', _) => Some(ChangeStatus::StagedModified),
        ('D', _) => Some(ChangeStatus::StagedDeleted),
        ('R', _) => Some(ChangeStatus::StagedRenamed),
        (' ', 'M') => Some(ChangeStatus::UnstagedModified),
        (' ', 'D') => Some(ChangeStatus::UnstagedDeleted),
        (' ', 'R') => Some(ChangeStatus::UnstagedRenamed),
        _ => None,
    }
}
