//! Reconciliation for clippy.toml's `msrv` key.

#![expect(
    clippy::type_complexity,
    reason = "Collected assertions are plainly Vec<(Provenance, A)> and per-key maps of them; the shapes are declared openly at every signature instead of hidden behind wrapper types or aliases (taxonomy decision 2026-06-07)."
)]
use std::collections::BTreeSet;

use aqc_file_engine_core::{Finding, Provenance, Severity, parse_version_tuple};
use toml_edit::{DocumentMut, Item, value};

use crate::reconcile::util::all_provenances;
use crate::requirement::MsrvAssertion;

/// Apply every `msrv` contribution to the document.
pub(crate) fn apply(
    doc: &mut DocumentMut,
    merged: &Vec<(Provenance, MsrvAssertion)>,
    findings: &mut Vec<Finding>,
) {
    let attribution = all_provenances(merged);
    let current = doc
        .get("msrv")
        .and_then(Item::as_str)
        .map(ToOwned::to_owned);

    for (_, assertion) in merged {
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
        MsrvAssertion::Equals(want, message) => {
            apply_equals(doc, current, want, message, attribution, findings);
        }
        MsrvAssertion::AtLeast(min, message) => {
            apply_at_least(doc, current, min, message, attribution, findings);
        }
        MsrvAssertion::OneOf(allowed, message) => {
            apply_one_of(current, allowed, message, attribution, findings);
        }
        MsrvAssertion::Present(message) => apply_present(current, message, attribution, findings),
        MsrvAssertion::Absent(message) => {
            apply_absent(doc, current, message, attribution, findings);
        }
    }
}

/// `msrv == want`.
fn apply_equals(
    doc: &mut DocumentMut,
    current: Option<&str>,
    want: &str,
    message: &str,
    attribution: &[Provenance],
    findings: &mut Vec<Finding>,
) {
    if current == Some(want) {
        return;
    }
    findings.push(Finding::Mismatch {
        key: "msrv".into(),
        current: current.map(ToOwned::to_owned),
        expected: want.to_owned(),
        message: message.to_owned(),
        severity: Severity::Error,
        attribution: attribution.to_vec(),
    });
    doc["msrv"] = value(want.to_owned());
}

/// `msrv >= min`.
fn apply_at_least(
    doc: &mut DocumentMut,
    current: Option<&str>,
    min: &str,
    message: &str,
    attribution: &[Provenance],
    findings: &mut Vec<Finding>,
) {
    if current.is_some_and(|c| ge_version(c, min)) {
        return;
    }
    findings.push(Finding::Mismatch {
        key: "msrv".into(),
        current: current.map(ToOwned::to_owned),
        expected: format!("at least {min}"),
        message: message.to_owned(),
        severity: Severity::Error,
        attribution: attribution.to_vec(),
    });
    doc["msrv"] = value(min.to_owned());
}

/// `msrv ∈ allowed`.
fn apply_one_of(
    current: Option<&str>,
    allowed: &BTreeSet<String>,
    message: &str,
    attribution: &[Provenance],
    findings: &mut Vec<Finding>,
) {
    if current.is_some_and(|c| allowed.iter().any(|a| a == c)) {
        return;
    }
    findings.push(Finding::Mismatch {
        key: "msrv".into(),
        current: current.map(ToOwned::to_owned),
        expected: format!("one of {allowed:?}"),
        message: message.to_owned(),
        severity: Severity::Error,
        attribution: attribution.to_vec(),
    });
}

/// `msrv` is set (any value).
fn apply_present(
    current: Option<&str>,
    message: &str,
    attribution: &[Provenance],
    findings: &mut Vec<Finding>,
) {
    if current.is_some() {
        return;
    }
    findings.push(Finding::Mismatch {
        key: "msrv".into(),
        current: None,
        expected: "any value (Present)".into(),
        message: message.to_owned(),
        severity: Severity::Error,
        attribution: attribution.to_vec(),
    });
}

/// `msrv` is not set.
fn apply_absent(
    doc: &mut DocumentMut,
    current: Option<&str>,
    message: &str,
    attribution: &[Provenance],
    findings: &mut Vec<Finding>,
) {
    if current.is_none() {
        return;
    }
    findings.push(Finding::Mismatch {
        key: "msrv".into(),
        current: current.map(ToOwned::to_owned),
        expected: "absent".into(),
        message: message.to_owned(),
        severity: Severity::Error,
        attribution: attribution.to_vec(),
    });
    let _ = doc.as_table_mut().remove("msrv");
}

/// Compare semver-ish dotted version strings.
fn ge_version(a: &str, b: &str) -> bool {
    parse_version_tuple(a) >= parse_version_tuple(b)
}
