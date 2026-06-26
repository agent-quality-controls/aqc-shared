//! Shared TOML finding helpers.

use aqc_file_engine_core::{Finding, Provenance, ResolvedRequirement, Severity};

/// Collect provenance from a resolved requirement.
#[must_use]
pub fn attribution<Merged, Assertion>(
    resolved: &ResolvedRequirement<Merged, Assertion>,
) -> Vec<Provenance> {
    resolved
        .collected
        .iter()
        .map(|(prov, _)| prov.clone())
        .collect()
}

/// Push a writable-key mismatch finding.
pub fn push_mismatch(
    findings: &mut Vec<Finding>,
    key: String,
    current: Option<String>,
    expected: String,
    message: String,
    attribution: &[Provenance],
) {
    findings.push(Finding::Mismatch {
        key,
        current,
        expected,
        message,
        severity: Severity::Error,
        attribution: attribution.to_vec(),
    });
}
