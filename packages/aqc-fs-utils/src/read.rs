//! The two read entry points.

use std::path::Path;

use crate::error::ReadError;
use crate::fs;
use crate::options::{ReadBytesOptions, ReadTextOptions};

/// Read a file as strict UTF-8 text.
///
/// Empty file is `Ok("")`. A raw `0x00` byte is rejected before decoding
/// (text in AQC repos must not contain NUL; treating it as an error matches
/// git/ripgrep-style binary detection).
///
/// # Errors
///
/// Every [`ReadError`] variant per the contract in `plan.md`.
pub fn read_text(path: impl AsRef<Path>, options: &ReadTextOptions) -> Result<String, ReadError> {
    let bytes = fs::read_capped(path.as_ref(), options.symlink, options.max_bytes)?;
    if bytes.contains(&0) {
        return Err(ReadError::ContainsNulByte);
    }
    let text = String::from_utf8(bytes).map_err(|_| ReadError::InvalidUtf8)?;
    if options.normalize_crlf {
        return Ok(text.replace("\r\n", "\n"));
    }
    Ok(text)
}

/// Read a file as raw bytes. No UTF-8 check, no NUL check.
///
/// # Errors
///
/// [`ReadError::NotFound`] / [`ReadError::NotAFile`] / [`ReadError::TooLarge`]
/// / [`ReadError::Io`].
pub fn read_bytes(
    path: impl AsRef<Path>,
    options: &ReadBytesOptions,
) -> Result<Vec<u8>, ReadError> {
    fs::read_capped(path.as_ref(), options.symlink, options.max_bytes)
}
