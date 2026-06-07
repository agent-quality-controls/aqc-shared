//! Read-only Git worktree state for Spec3 lock/verify.
//!
//! Runs `git status --porcelain=v1 -z` as a subprocess and parses the
//! NUL-separated records. No commits, merges, or object-database access.
//! Disk existence is `aqc-filetree`; Git change detection is this crate.
//! Contract: `plan.md` in this directory.

#![expect(
    clippy::type_complexity,
    reason = "Result<Vec<...>, Error> return shapes exceed the strict workspace threshold; the shapes are the crate's declared contract, stated openly rather than aliased away."
)]

// Dev-dependency linked into the lib's test build but exercised only by the
// integration tests in `tests/`.
#[cfg(test)]
use tempfile as _;

mod error;
mod git;
mod porcelain;
mod queries;
mod status;

#[cfg(feature = "api")]
pub use error::GitError;
#[cfg(feature = "api")]
pub use git::worktree_changes;
#[cfg(feature = "api")]
pub use porcelain::parse_porcelain_v1z;
#[cfg(feature = "api")]
pub use queries::changes_affecting_paths;
#[cfg(feature = "api")]
pub use queries::dirty_paths;
#[cfg(feature = "api")]
pub use queries::is_worktree_clean;
#[cfg(feature = "api")]
pub use status::ChangeStatus;
#[cfg(feature = "api")]
pub use status::ColumnChange;
#[cfg(feature = "api")]
pub use status::PORCELAIN_VERSION;
#[cfg(feature = "api")]
pub use status::PorcelainOptions;
#[cfg(feature = "api")]
pub use status::WorktreeChange;
