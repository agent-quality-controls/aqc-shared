//! Reconciliation for clippy.toml's `msrv` key.

use aqc_file_engine_core::{Finding, MergedAssertion, Provenance, Severity};
use toml_edit::{DocumentMut, Item, value};

use crate::reconcile::util::all_provenances;
use crate::requirement::MsrvAssertion;

/// Apply every `msrv` contribution to the document.
pub(crate) fn apply_msrv(
    doc: &mut DocumentMut,
    merged: &MergedAssertion<MsrvAssertion>,
    findings: &mut Vec<Finding>,
) {
    let attribution = all_provenances(merged);
    let current = doc
        .get("msrv")
        .and_then(Item::as_str)
        .map(ToOwned::to_owned);

    for (_, assertion) in &merged.contributions {
        apply_one(doc, current.as_deref(), assertion, &attribution, findings);
    }
}

/// Apply a single `MsrvAssertion` against the current on-disk value.
fn apply_one(
    doc: &mut DocumentMut,
    current: Option<&str>,
    assertion: &MsrvAssertion,
    attribution: &[Provenance],
    findings: &mut Vec<Finding>,
) {
    match assertion {
        MsrvAssertion::Equals(want) => apply_equals(doc, current, want, attribution, findings),
        MsrvAssertion::AtLeast(min) => apply_at_least(doc, current, min, attribution, findings),
        MsrvAssertion::OneOf(allowed) => apply_one_of(current, allowed, attribution, findings),
        MsrvAssertion::Present => apply_present(current, attribution, findings),
        MsrvAssertion::Absent => apply_absent(doc, current, attribution, findings),
    }
}

/// Enforce `msrv == want`.
fn apply_equals(
    doc: &mut DocumentMut,
    current: Option<&str>,
    want: &str,
    attribution: &[Provenance],
    findings: &mut Vec<Finding>,
) {
    if current == Some(want) {
        return;
    }
    findings.push(Finding::Mismatch {
        path: "msrv".into(),
        current: current.map(ToOwned::to_owned),
        expected: want.to_owned(),
        severity: Severity::Error,
        attribution: attribution.to_vec(),
    });
    doc["msrv"] = value(want.to_owned());
}

/// Enforce `msrv >= min`.
fn apply_at_least(
    doc: &mut DocumentMut,
    current: Option<&str>,
    min: &str,
    attribution: &[Provenance],
    findings: &mut Vec<Finding>,
) {
    if current.is_some_and(|c| ge_version(c, min)) {
        return;
    }
    findings.push(Finding::Mismatch {
        path: "msrv".into(),
        current: current.map(ToOwned::to_owned),
        expected: format!("at least {min}"),
        severity: Severity::Error,
        attribution: attribution.to_vec(),
    });
    doc["msrv"] = value(min.to_owned());
}

/// Enforce `msrv ∈ allowed`.
fn apply_one_of(
    current: Option<&str>,
    allowed: &std::collections::BTreeSet<String>,
    attribution: &[Provenance],
    findings: &mut Vec<Finding>,
) {
    if current.is_some_and(|c| allowed.iter().any(|a| a == c)) {
        return;
    }
    findings.push(Finding::Mismatch {
        path: "msrv".into(),
        current: current.map(ToOwned::to_owned),
        expected: format!("one of {allowed:?}"),
        severity: Severity::Error,
        attribution: attribution.to_vec(),
    });
}

/// Enforce `msrv` is set (any value).
fn apply_present(current: Option<&str>, attribution: &[Provenance], findings: &mut Vec<Finding>) {
    if current.is_some() {
        return;
    }
    findings.push(Finding::Mismatch {
        path: "msrv".into(),
        current: None,
        expected: "any value (Present)".into(),
        severity: Severity::Error,
        attribution: attribution.to_vec(),
    });
}

/// Enforce `msrv` is not set.
fn apply_absent(
    doc: &mut DocumentMut,
    current: Option<&str>,
    attribution: &[Provenance],
    findings: &mut Vec<Finding>,
) {
    if current.is_none() {
        return;
    }
    findings.push(Finding::Mismatch {
        path: "msrv".into(),
        current: current.map(ToOwned::to_owned),
        expected: "absent".into(),
        severity: Severity::Error,
        attribution: attribution.to_vec(),
    });
    let _ = doc.as_table_mut().remove("msrv");
}

/// Compare semver-ish dotted version strings (`1.85`, `1.85.0`). Returns
/// `true` when `a >= b`. Treats missing parts as 0.
fn ge_version(a: &str, b: &str) -> bool {
    parse_version(a) >= parse_version(b)
}

/// Parse a dotted version string into a tuple, treating missing parts as 0.
fn parse_version(v: &str) -> (u64, u64, u64) {
    let mut parts = v.split('.').map(|p| p.parse::<u64>().unwrap_or(0));
    (
        parts.next().unwrap_or(0),
        parts.next().unwrap_or(0),
        parts.next().unwrap_or(0),
    )
}
