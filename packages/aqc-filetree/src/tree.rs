//! The walk result: `FileTree`, its entries, and the queries.

use std::path::PathBuf;

use globset::{GlobBuilder, GlobSetBuilder};

/// What a tree entry is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileKind {
    /// Regular file.
    File,
    /// Directory.
    Directory,
    /// Symlink (recorded only under [`SymlinkPolicy::Record`]).
    Symlink,
}

/// Which phase produced an entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntryOrigin {
    /// Phase 1 walk.
    Primary,
    /// Phase 2 recovery walk.
    Recovered,
}

/// One entry of a [`FileTree`].
#[derive(Debug, Clone)]
pub struct FileEntry {
    /// Relative to the walk root: `/` separators, UTF-8, no leading `/`, no `..`.
    pub rel_path: String,
    /// Absolute path.
    pub abs_path: PathBuf,
    /// What the entry is.
    pub kind: FileKind,
    /// Which phase produced it.
    pub origin: EntryOrigin,
}

/// The result of a walk: the root plus entries sorted by `rel_path`.
#[derive(Debug, Clone)]
pub struct FileTree {
    /// Walk root (canonicalized when possible).
    pub root: PathBuf,
    /// Entries sorted by `rel_path`.
    pub entries: Vec<FileEntry>,
}

impl FileTree {
    /// The entry at exactly this `rel_path`, if present.
    #[must_use]
    pub fn entry(&self, rel_path: &str) -> Option<&FileEntry> {
        self.entries
            .binary_search_by(|e| e.rel_path.as_str().cmp(rel_path))
            .ok()
            .and_then(|i| self.entries.get(i))
    }

    /// All entries with the given origin.
    #[must_use]
    pub fn entries_with_origin(&self, origin: EntryOrigin) -> Vec<&FileEntry> {
        self.entries.iter().filter(|e| e.origin == origin).collect()
    }

    /// Entries whose `rel_path` matches the glob pattern.
    ///
    /// # Errors
    ///
    /// Returns the `globset` error for an invalid pattern.
    pub fn glob(
        &self,
        pattern: &str,
        case_sensitive: bool,
    ) -> Result<Vec<&FileEntry>, globset::Error> {
        let glob = GlobBuilder::new(pattern)
            .case_insensitive(!case_sensitive)
            .literal_separator(false)
            .build()?;
        let mut builder = GlobSetBuilder::new();
        let _ = builder.add(glob);
        let set = builder.build()?;
        Ok(self
            .entries
            .iter()
            .filter(|e| set.is_match(&e.rel_path))
            .collect())
    }
}
