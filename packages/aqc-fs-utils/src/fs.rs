//! The crate's filesystem boundary: every `std::fs` touch lives here.

#![expect(
    clippy::disallowed_methods,
    clippy::disallowed_types,
    reason = "This crate IS the centralized fs layer the workspace-wide std::fs bans route everyone to; the touches are confined to this boundary module."
)]

use std::fs;
use std::io::Read as _;
use std::path::Path;

use crate::error::ReadError;
use crate::options::SymlinkReadPolicy;

/// Shared stat + cap + read body.
pub(crate) fn read_capped(
    path: &Path,
    symlink: SymlinkReadPolicy,
    max_bytes: u64,
) -> Result<Vec<u8>, ReadError> {
    let metadata = match symlink {
        SymlinkReadPolicy::DontFollow => fs::symlink_metadata(path),
        SymlinkReadPolicy::Follow => fs::metadata(path),
    };
    let metadata = metadata.map_err(|source| classify_io(path, source))?;
    if !metadata.is_file() {
        return Err(ReadError::NotAFile);
    }
    if metadata.len() > max_bytes {
        return Err(ReadError::TooLarge);
    }
    let file = fs::File::open(path).map_err(|source| classify_io(path, source))?;
    let mut bytes = Vec::new();
    // Cap the reader as well: the stat is a TOCTOU estimate, the cap is the law.
    let read = file
        .take(max_bytes.saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(|source| classify_io(path, source))?;
    if u64::try_from(read).unwrap_or(u64::MAX) > max_bytes {
        return Err(ReadError::TooLarge);
    }
    Ok(bytes)
}

/// Map an OS error to `NotFound` or `Io`.
fn classify_io(path: &Path, source: std::io::Error) -> ReadError {
    if source.kind() == std::io::ErrorKind::NotFound {
        return ReadError::NotFound;
    }
    ReadError::Io {
        path: path.to_path_buf(),
        source,
    }
}
