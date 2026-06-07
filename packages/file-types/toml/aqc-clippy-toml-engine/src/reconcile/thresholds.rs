//! Reconciliation for clippy.toml's numeric threshold keys.

#![expect(
    clippy::type_complexity,
    reason = "Collected assertions are plainly Vec<(Provenance, A)> and per-key maps of them; the shapes are declared openly at every signature instead of hidden behind wrapper types or aliases (taxonomy decision 2026-06-07)."
)]
use std::collections::BTreeMap;

use aqc_file_engine_core::{Finding, Provenance, Severity};
use toml_edit::{DocumentMut, Item, value};

use crate::reconcile::util::all_provenances;
use crate::requirement::ThresholdsAssertion;

/// Apply every thresholds contribution to the document.
pub(crate) fn apply(
    doc: &mut DocumentMut,
    merged: &Vec<(Provenance, ThresholdsAssertion)>,
    findings: &mut Vec<Finding>,
) {
    let attribution = all_provenances(merged);
    for (_, assertion) in merged {
        apply_one(doc, assertion, &attribution, findings);
    }
}

/// Dispatch one `ThresholdsAssertion`.
fn apply_one(
    doc: &mut DocumentMut,
    assertion: &ThresholdsAssertion,
    attribution: &[Provenance],
    findings: &mut Vec<Finding>,
) {
    match assertion {
        ThresholdsAssertion::Equals(map) => {
            apply_map(doc, map, attribution, findings, apply_equals);
        }
        ThresholdsAssertion::AtMost(map) => {
            apply_map(doc, map, attribution, findings, apply_at_most);
        }
        ThresholdsAssertion::AtLeast(map) => {
            apply_map(doc, map, attribution, findings, apply_at_least);
        }
        ThresholdsAssertion::Present(map) => apply_present(doc, map, attribution, findings),
        ThresholdsAssertion::Absent(map) => apply_absent(doc, map, attribution, findings),
    }
}

/// Walk a `(key, (value, message))` map and apply the per-key handler to each.
fn apply_map(
    doc: &mut DocumentMut,
    map: &BTreeMap<String, (u64, String)>,
    attribution: &[Provenance],
    findings: &mut Vec<Finding>,
    handler: fn(&mut DocumentMut, &str, u64, &str, &[Provenance], &mut Vec<Finding>),
) {
    for (key, (want, message)) in map {
        handler(doc, key, *want, message, attribution, findings);
    }
}

/// Each named key must be present with an integer value.
fn apply_present(
    doc: &DocumentMut,
    map: &BTreeMap<String, String>,
    attribution: &[Provenance],
    findings: &mut Vec<Finding>,
) {
    for (key, message) in map {
        if doc.get(key).and_then(Item::as_integer).is_some() {
            continue;
        }
        findings.push(Finding::Mismatch {
            key: key.clone(),
            current: None,
            expected: "any integer (Present)".into(),
            message: message.clone(),
            severity: Severity::Error,
            attribution: attribution.to_vec(),
        });
    }
}

/// Each named key must not exist.
fn apply_absent(
    doc: &mut DocumentMut,
    map: &BTreeMap<String, String>,
    attribution: &[Provenance],
    findings: &mut Vec<Finding>,
) {
    for (key, message) in map {
        if !doc.contains_key(key) {
            continue;
        }
        findings.push(Finding::Mismatch {
            key: key.clone(),
            current: doc
                .get(key)
                .and_then(Item::as_integer)
                .map(|n| n.to_string()),
            expected: "absent".into(),
            message: message.clone(),
            severity: Severity::Error,
            attribution: attribution.to_vec(),
        });
        let _ = doc.as_table_mut().remove(key);
    }
}

/// `key == want`.
fn apply_equals(
    doc: &mut DocumentMut,
    key: &str,
    want: u64,
    message: &str,
    attribution: &[Provenance],
    findings: &mut Vec<Finding>,
) {
    let current = doc.get(key).and_then(Item::as_integer);
    let want_i64 = i64::try_from(want).ok();
    if current == want_i64 {
        return;
    }
    findings.push(Finding::Mismatch {
        key: key.into(),
        current: current.map(|n| n.to_string()),
        expected: want.to_string(),
        message: message.to_owned(),
        severity: Severity::Error,
        attribution: attribution.to_vec(),
    });
    if let Some(n) = want_i64 {
        doc[key] = value(n);
    }
}

/// `key <= ceiling`.
fn apply_at_most(
    doc: &mut DocumentMut,
    key: &str,
    ceiling: u64,
    message: &str,
    attribution: &[Provenance],
    findings: &mut Vec<Finding>,
) {
    let current = doc.get(key).and_then(Item::as_integer);
    let ceiling_i64 = i64::try_from(ceiling).unwrap_or(i64::MAX);
    if current.is_some_and(|c| c <= ceiling_i64) {
        return;
    }
    findings.push(Finding::Mismatch {
        key: key.into(),
        current: current.map(|n| n.to_string()),
        expected: format!("at most {ceiling}"),
        message: message.to_owned(),
        severity: Severity::Error,
        attribution: attribution.to_vec(),
    });
    doc[key] = value(ceiling_i64);
}

/// `key >= floor`.
fn apply_at_least(
    doc: &mut DocumentMut,
    key: &str,
    floor: u64,
    message: &str,
    attribution: &[Provenance],
    findings: &mut Vec<Finding>,
) {
    let current = doc.get(key).and_then(Item::as_integer);
    let floor_i64 = i64::try_from(floor).unwrap_or(0);
    if current.is_some_and(|c| c >= floor_i64) {
        return;
    }
    findings.push(Finding::Mismatch {
        key: key.into(),
        current: current.map(|n| n.to_string()),
        expected: format!("at least {floor}"),
        message: message.to_owned(),
        severity: Severity::Error,
        attribution: attribution.to_vec(),
    });
    doc[key] = value(floor_i64);
}
