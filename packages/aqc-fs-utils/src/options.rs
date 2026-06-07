//! Read options and the symlink policy.

/// How a symlink at the given path is treated when opening.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SymlinkReadPolicy {
    /// Open the symlink node itself (a symlink is then `NotAFile`).
    #[default]
    DontFollow,
    /// Open the target.
    Follow,
}

/// Options for [`read_text`].
#[derive(Debug, Clone)]
pub struct ReadTextOptions {
    /// Symlink handling at the path.
    pub symlink: SymlinkReadPolicy,
    /// Files larger than this are rejected with [`ReadError::TooLarge`].
    pub max_bytes: u64,
    /// Replace `\r\n` with `\n` in the returned string.
    pub normalize_crlf: bool,
}

impl Default for ReadTextOptions {
    fn default() -> Self {
        Self {
            symlink: SymlinkReadPolicy::DontFollow,
            max_bytes: MAX_BYTES_DEFAULT,
            normalize_crlf: false,
        }
    }
}

/// Options for [`read_bytes`].
#[derive(Debug, Clone)]
pub struct ReadBytesOptions {
    /// Symlink handling at the path.
    pub symlink: SymlinkReadPolicy,
    /// Files larger than this are rejected with [`ReadError::TooLarge`].
    pub max_bytes: u64,
}

impl Default for ReadBytesOptions {
    fn default() -> Self {
        Self {
            symlink: SymlinkReadPolicy::DontFollow,
            max_bytes: MAX_BYTES_DEFAULT,
        }
    }
}

/// Default size cap: 1 GiB.
const MAX_BYTES_DEFAULT: u64 = 1_073_741_824;
