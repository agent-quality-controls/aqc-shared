//! Reconciliation for clippy.toml string-valued (enum-style) settings.

#![expect(
    clippy::type_complexity,
    reason = "Collected assertions are plainly Vec<(Provenance, A)> and per-key maps of them; the shapes are declared openly at every signature instead of hidden behind wrapper types or aliases (taxonomy decision 2026-06-07)."
)]
use std::collections::{BTreeMap, BTreeSet};

use aqc_file_engine_core::{Finding, Provenance, Severity};
use toml_edit::{DocumentMut, Item, value};

use crate::reconcile::util::all_provenances;
use crate::requirement::StringAssertion;

/// Apply every string-setting contribution.
pub(crate) fn apply(
    doc: &mut DocumentMut,
    merged_by_key: &BTreeMap<String, Vec<(Provenance, StringAssertion)>>,
    findings: &mut Vec<Finding>,
) {
    for (key, merged) in merged_by_key {
        let attribution = all_provenances(merged);
        for (_, assertion) in merged {
            apply_one(doc, key, assertion, &attribution, findings);
        }
    }
}

/// Apply a single `StringAssertion` against a setting.
fn apply_one(
    doc: &mut DocumentMut,
    key: &str,
    assertion: &StringAssertion,
    attribution: &[Provenance],
    findings: &mut Vec<Finding>,
) {
    match assertion {
        StringAssertion::Equals(want, message) => {
            apply_equals(doc, key, want, message, attribution, findings);
        }
        StringAssertion::OneOf(allowed, message) => {
            apply_one_of(doc, key, allowed, message, attribution, findings);
        }
        StringAssertion::Present(message) => {
            apply_present(doc, key, message, attribution, findings);
        }
        StringAssertion::Absent(message) => apply_absent(doc, key, message, attribution, findings),
    }
}

/// `key == want`.
fn apply_equals(
    doc: &mut DocumentMut,
    key: &str,
    want: &str,
    message: &str,
    attribution: &[Provenance],
    findings: &mut Vec<Finding>,
) {
    let current = doc.get(key).and_then(Item::as_str).map(ToOwned::to_owned);
    if current.as_deref() == Some(want) {
        return;
    }
    findings.push(Finding::Mismatch {
        key: key.into(),
        current,
        expected: want.to_owned(),
        message: message.to_owned(),
        severity: Severity::Error,
        attribution: attribution.to_vec(),
    });
    doc[key] = value(want.to_owned());
}

/// `key ∈ allowed`.
fn apply_one_of(
    doc: &DocumentMut,
    key: &str,
    allowed: &BTreeSet<String>,
    message: &str,
    attribution: &[Provenance],
    findings: &mut Vec<Finding>,
) {
    let current = doc.get(key).and_then(Item::as_str);
    if current.is_some_and(|c| allowed.contains(c)) {
        return;
    }
    findings.push(Finding::Mismatch {
        key: key.into(),
        current: current.map(ToOwned::to_owned),
        expected: format!("one of {allowed:?}"),
        message: message.to_owned(),
        severity: Severity::Error,
        attribution: attribution.to_vec(),
    });
}

/// `key` must be set with any string.
fn apply_present(
    doc: &DocumentMut,
    key: &str,
    message: &str,
    attribution: &[Provenance],
    findings: &mut Vec<Finding>,
) {
    if doc.get(key).and_then(Item::as_str).is_some() {
        return;
    }
    findings.push(Finding::Mismatch {
        key: key.into(),
        current: None,
        expected: "any string (Present)".into(),
        message: message.to_owned(),
        severity: Severity::Error,
        attribution: attribution.to_vec(),
    });
}

/// `key` must not be set.
fn apply_absent(
    doc: &mut DocumentMut,
    key: &str,
    message: &str,
    attribution: &[Provenance],
    findings: &mut Vec<Finding>,
) {
    if !doc.contains_key(key) {
        return;
    }
    findings.push(Finding::Mismatch {
        key: key.into(),
        current: doc.get(key).and_then(Item::as_str).map(ToOwned::to_owned),
        expected: "absent".into(),
        message: message.to_owned(),
        severity: Severity::Error,
        attribution: attribution.to_vec(),
    });
    let _ = doc.as_table_mut().remove(key);
}
