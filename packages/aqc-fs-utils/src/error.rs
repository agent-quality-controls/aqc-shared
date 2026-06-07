//! The read error contract.

use std::path::PathBuf;

/// Why a read failed.
#[derive(Debug)]
pub enum ReadError {
    /// The path does not exist.
    NotFound,
    /// The path is not a regular file (directory, unfollowed symlink, ...).
    NotAFile,
    /// The file is larger than `max_bytes`.
    TooLarge,
    /// The raw bytes contain `0x00` (text reads only).
    ContainsNulByte,
    /// The bytes are not valid UTF-8 (text reads only).
    InvalidUtf8,
    /// An OS error other than "not found".
    Io {
        /// The path the OS error occurred on.
        path: PathBuf,
        /// The underlying error.
        source: std::io::Error,
    },
}

impl core::fmt::Display for ReadError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::NotFound => write!(f, "file not found"),
            Self::NotAFile => write!(f, "not a regular file"),
            Self::TooLarge => write!(f, "file exceeds the size cap"),
            Self::ContainsNulByte => write!(f, "file contains a NUL byte"),
            Self::InvalidUtf8 => write!(f, "file is not valid UTF-8"),
            Self::Io { path, source } => write!(f, "io error on {}: {source}", path.display()),
        }
    }
}

impl core::error::Error for ReadError {}
