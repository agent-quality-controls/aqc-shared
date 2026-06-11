//! Read a single file from disk with fixed rules.
//!
//! Shared by Specular, Fixture3, Guardrail3. Not a repo walk (`aqc-filetree`),
//! not Git, not substring checks. All failures are `Err`: there is no `Ok`
//! branch for "binary" or "skip". Contract: `plan.md` in this directory.

#![expect(
    clippy::type_complexity,
    reason = "Result<Vec<...>, Error> return shapes exceed the strict workspace threshold; the shapes are the crate's declared contract, stated openly rather than aliased away."
)]

// Dev-dependency linked into the lib's test build but exercised only by the
// integration tests in `tests/`.
#[cfg(test)]
use tempfile as _;

mod error;
mod fs;
mod options;
mod read;

#[cfg(feature = "api")]
pub use error::ReadError;
#[cfg(feature = "api")]
pub use options::ReadBytesOptions;
#[cfg(feature = "api")]
pub use options::ReadTextOptions;
#[cfg(feature = "api")]
pub use options::SymlinkReadPolicy;
#[cfg(feature = "api")]
pub use read::read_bytes;
#[cfg(feature = "api")]
pub use read::read_text;
