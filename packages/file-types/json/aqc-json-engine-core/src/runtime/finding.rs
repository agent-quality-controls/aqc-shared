use aqc_file_engine_core::{Finding, Provenance, ResolvedRequirement, Severity};

#[must_use]
pub fn attribution<Merged, Assertion>(
    resolved: &ResolvedRequirement<Merged, Assertion>,
) -> Vec<Provenance> {
    resolved
        .collected
        .iter()
        .map(|(provenance, _)| provenance.clone())
        .collect()
}

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
        selector: None,
        current,
        expected,
        message,
        severity: Severity::Error,
        attribution: attribution.to_vec(),
    });
}
