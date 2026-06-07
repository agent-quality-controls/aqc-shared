//! The git query error contract.

/// Why a git query failed.
#[derive(Debug)]
pub enum GitError {
    /// The directory is not inside a Git worktree.
    NotARepository,
    /// The `git` executable is missing.
    GitNotInstalled,
    /// `git` exited nonzero.
    CommandFailed {
        /// The command that failed.
        command: String,
        /// Its stderr.
        stderr: String,
    },
    /// Unparseable porcelain output.
    ParseError {
        /// What could not be parsed.
        message: String,
    },
}

impl core::fmt::Display for GitError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::NotARepository => write!(f, "not a git repository"),
            Self::GitNotInstalled => write!(f, "git executable not found"),
            Self::CommandFailed { command, stderr } => {
                write!(f, "`{command}` failed: {stderr}")
            }
            Self::ParseError { message } => write!(f, "porcelain parse error: {message}"),
        }
    }
}

impl core::error::Error for GitError {}
