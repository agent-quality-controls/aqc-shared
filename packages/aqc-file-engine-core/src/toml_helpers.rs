//! Helpers every TOML-targeted engine needs.
//!
//! These don't perform I/O: they accept bytes that someone else read,
//! return a parsed document plus any `ParseError` findings.

use toml_edit::DocumentMut;

use crate::finding::Finding;
use crate::types::Severity;

/// Parse `current_bytes` into a `DocumentMut`, returning the document plus
/// any `Finding::ParseError` produced for invalid UTF-8 or invalid TOML.
///
/// `file_label` is used to name the file in the finding message
/// (e.g. `"Cargo.toml"` or `"clippy.toml"`).
///
/// On parse failure, returns an empty `DocumentMut` plus an
/// `Error`-severity finding. The engine's caller (init) sees the error
/// finding and refuses to write; validate reports it.
#[must_use]
#[expect(
    clippy::type_complexity,
    reason = "Returning (DocumentMut, Vec<Finding>) is the natural shape for this helper."
)]
pub fn parse_or_report(
    current_bytes: Option<&[u8]>,
    file_label: &str,
) -> (DocumentMut, Vec<Finding>) {
    let mut findings: Vec<Finding> = Vec::new();
    let text = match current_bytes {
        Some(bytes) => match std::str::from_utf8(bytes) {
            Ok(s) => s,
            Err(e) => {
                findings.push(Finding::ParseError {
                    message: format!("{file_label} is not valid UTF-8: {e}"),
                    severity: Severity::Error,
                });
                return (DocumentMut::new(), findings);
            }
        },
        None => "",
    };
    match text.parse::<DocumentMut>() {
        Ok(doc) => (doc, findings),
        Err(e) => {
            findings.push(Finding::ParseError {
                message: format!("{file_label} is not valid TOML: {e}"),
                severity: Severity::Error,
            });
            (DocumentMut::new(), findings)
        }
    }
}

/// Parse a dotted version string (`1.85`, `1.85.0`, `v1.85.0`) into a
/// `(major, minor, patch)` tuple. Treats missing parts as 0 and
/// non-numeric parts as 0.
#[must_use]
pub fn parse_version_tuple(v: &str) -> (u64, u64, u64) {
    let normalized = v.trim_start_matches('v');
    let mut parts = normalized.split('.').map(|p| p.parse::<u64>().unwrap_or(0));
    (
        parts.next().unwrap_or(0),
        parts.next().unwrap_or(0),
        parts.next().unwrap_or(0),
    )
}
