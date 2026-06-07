//! The crate's subprocess boundary: every `git` invocation lives here.

#![expect(
    clippy::disallowed_methods,
    reason = "Running git as a subprocess is this crate's purpose; the invocations are confined to this boundary module."
)]

use std::path::Path;
use std::process::Command;

use crate::error::GitError;
use crate::porcelain::parse_porcelain_v1z;
use crate::status::{ChangeStatus, PorcelainOptions, WorktreeChange};

/// Run porcelain status at `repo_root` and return all changes.
///
/// # Errors
///
/// [`GitError`] per the contract in `plan.md`.
pub fn worktree_changes(
    repo_root: impl AsRef<Path>,
    options: PorcelainOptions,
) -> Result<Vec<WorktreeChange>, GitError> {
    let repo_root = repo_root.as_ref();
    let mut command = Command::new("git");
    // Pin the locale: classification must not depend on translated output.
    let _ = command.env("LC_ALL", "C");
    let _ = command
        .arg("-C")
        .arg(repo_root)
        .args(["status", "--porcelain=v1", "-z"]);
    if options.include_ignored {
        let _ = command.arg("--ignored");
    }
    let output = command.output().map_err(|source| {
        if source.kind() == std::io::ErrorKind::NotFound {
            GitError::GitNotInstalled
        } else {
            GitError::CommandFailed {
                command: "git status --porcelain=v1 -z".to_owned(),
                stderr: source.to_string(),
            }
        }
    })?;
    if !output.status.success() {
        // Decide repo-ness structurally, never by stderr text.
        if !is_inside_work_tree(repo_root) {
            return Err(GitError::NotARepository);
        }
        return Err(GitError::CommandFailed {
            command: "git status --porcelain=v1 -z".to_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        });
    }
    let text = String::from_utf8(output.stdout).map_err(|_| GitError::ParseError {
        message: "porcelain output is not UTF-8".to_owned(),
    })?;
    let changes = parse_porcelain_v1z(&text)?;
    Ok(changes
        .into_iter()
        .filter(|c| match c.status {
            ChangeStatus::Untracked => options.include_untracked,
            ChangeStatus::Ignored => options.include_ignored,
            ChangeStatus::Tracked { .. } | ChangeStatus::Conflicted => true,
        })
        .collect())
}

/// True when `git rev-parse --is-inside-work-tree` answers `true`.
fn is_inside_work_tree(repo_root: &Path) -> bool {
    let mut command = Command::new("git");
    let _ = command.env("LC_ALL", "C");
    let output = command
        .arg("-C")
        .arg(repo_root)
        .args(["rev-parse", "--is-inside-work-tree"])
        .output();
    output.is_ok_and(|out| out.status.success() && out.stdout.starts_with(b"true"))
}
