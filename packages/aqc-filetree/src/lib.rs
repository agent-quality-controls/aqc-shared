//! Walk one filesystem directory root, return a [`FileTree`].
//!
//! No Git commands, no file reads, no parsing, no product policy. Two phases:
//! phase 1 is gitignore-aware; phase 2 (opt-in via [`RecoveryRules`]) walks
//! WITHOUT gitignore and recovers specific paths from ignored trees, tagged
//! [`EntryOrigin::Recovered`]. Contract: `plan.md` in this directory.

#![expect(
    clippy::type_complexity,
    reason = "Result<Vec<...>, Error> return shapes exceed the strict workspace threshold; the shapes are the crate's declared contract, stated openly rather than aliased away."
)]

// Dev-dependency linked into the lib's test build but exercised only by the
// integration tests in `tests/`.
#[cfg(test)]
use tempfile as _;

mod fs;
mod options;
mod tree;
mod walk;

#[cfg(feature = "api")]
pub use options::RecoveryRules;
#[cfg(feature = "api")]
pub use options::SkipDirPreset;
#[cfg(feature = "api")]
pub use options::SymlinkPolicy;
#[cfg(feature = "api")]
pub use options::WalkError;
#[cfg(feature = "api")]
pub use options::WalkOptions;
#[cfg(feature = "api")]
pub use tree::EntryOrigin;
#[cfg(feature = "api")]
pub use tree::FileEntry;
#[cfg(feature = "api")]
pub use tree::FileKind;
#[cfg(feature = "api")]
pub use tree::FileTree;
#[cfg(feature = "api")]
pub use walk::build_file_tree;
