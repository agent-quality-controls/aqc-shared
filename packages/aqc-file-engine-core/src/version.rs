//! Shared version helpers.

/// Parse a dotted version string (`1.85`, `1.85.0`, `v1.85.0`) into a
/// `(major, minor, patch)` tuple.
///
/// Missing parts and non-numeric parts compare as 0.
#[must_use]
pub fn parse_version_tuple(value: &str) -> (u64, u64, u64) {
    let normalized = value.trim_start_matches('v');
    let mut parts = normalized
        .split('.')
        .map(|part| part.parse::<u64>().unwrap_or(0));
    (
        parts.next().unwrap_or(0),
        parts.next().unwrap_or(0),
        parts.next().unwrap_or(0),
    )
}
