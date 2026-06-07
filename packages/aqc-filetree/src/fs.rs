//! The crate's filesystem boundary: every direct `std::fs` touch lives here
//! (the walk itself goes through the `ignore` crate).

#![expect(
    clippy::disallowed_methods,
    reason = "This crate IS the centralized walk layer; the std::fs touches are confined to this boundary module."
)]

use std::path::{Path, PathBuf};

use crate::options::WalkError;

/// Validate the walk root and canonicalize it when possible.
pub(crate) fn checked_root(root: &Path) -> Result<PathBuf, WalkError> {
    let metadata = std::fs::metadata(root).map_err(|source| {
        if source.kind() == std::io::ErrorKind::NotFound {
            WalkError::RootNotFound
        } else {
            WalkError::Io {
                path: root.to_path_buf(),
                source,
            }
        }
    })?;
    if !metadata.is_dir() {
        return Err(WalkError::RootNotADirectory);
    }
    Ok(root.canonicalize().unwrap_or_else(|_| root.to_path_buf()))
}
