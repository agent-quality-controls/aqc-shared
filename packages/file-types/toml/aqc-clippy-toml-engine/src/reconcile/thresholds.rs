//! Reconciliation for clippy.toml's numeric threshold keys.

use std::collections::{BTreeMap, BTreeSet};

use aqc_file_engine_core::{Finding, MergedAssertion, Provenance, Severity};
use toml_edit::{DocumentMut, Item, value};

use crate::reconcile::util::all_provenances;
use crate::requirement::ThresholdsAssertion;

/// Apply every thresholds contribution to the document.
pub(crate) fn apply_thresholds(
    doc: &mut DocumentMut,
    merged: &MergedAssertion<ThresholdsAssertion>,
    findings: &mut Vec<Finding>,
) {
    let attribution = all_provenances(merged);
    for (_, assertion) in &merged.contributions {
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
        ThresholdsAssertion::Equals(map) => apply_map_equals(doc, map, attribution, findings),
        ThresholdsAssertion::AtMost(map) => apply_map_at_most(doc, map, attribution, findings),
        ThresholdsAssertion::AtLeast(map) => apply_map_at_least(doc, map, attribution, findings),
        ThresholdsAssertion::Present(keys) => apply_present(doc, keys, attribution, findings),
        ThresholdsAssertion::Absent(keys) => apply_absent(doc, keys, attribution, findings),
    }
}

/// Each `(key, value)` must equal exactly.
fn apply_map_equals(
    doc: &mut DocumentMut,
    map: &BTreeMap<String, u64>,
    attribution: &[Provenance],
    findings: &mut Vec<Finding>,
) {
    for (key, want) in map {
        apply_equals(doc, key, *want, attribution, findings);
    }
}

/// Each `(key, value)` is an upper bound.
fn apply_map_at_most(
    doc: &mut DocumentMut,
    map: &BTreeMap<String, u64>,
    attribution: &[Provenance],
    findings: &mut Vec<Finding>,
) {
    for (key, ceiling) in map {
        apply_at_most(doc, key, *ceiling, attribution, findings);
    }
}

/// Each `(key, value)` is a lower bound.
fn apply_map_at_least(
    doc: &mut DocumentMut,
    map: &BTreeMap<String, u64>,
    attribution: &[Provenance],
    findings: &mut Vec<Finding>,
) {
    for (key, floor) in map {
        apply_at_least(doc, key, *floor, attribution, findings);
    }
}

/// Each named key must be present with an integer value.
fn apply_present(
    doc: &DocumentMut,
    keys: &BTreeSet<String>,
    attribution: &[Provenance],
    findings: &mut Vec<Finding>,
) {
    for key in keys {
        if doc.get(key).and_then(Item::as_integer).is_some() {
            continue;
        }
        findings.push(Finding::Mismatch {
            path: key.clone(),
            current: None,
            expected: "any integer (Present)".into(),
            severity: Severity::Error,
            attribution: attribution.to_vec(),
        });
    }
}

/// Each named key must not exist.
fn apply_absent(
    doc: &mut DocumentMut,
    keys: &BTreeSet<String>,
    attribution: &[Provenance],
    findings: &mut Vec<Finding>,
) {
    for key in keys {
        if !doc.contains_key(key) {
            continue;
        }
        findings.push(Finding::Mismatch {
            path: key.clone(),
            current: doc
                .get(key)
                .and_then(Item::as_integer)
                .map(|n| n.to_string()),
            expected: "absent".into(),
            severity: Severity::Error,
            attribution: attribution.to_vec(),
        });
        let _ = doc.as_table_mut().remove(key);
    }
}

/// Apply a single scalar `Equals` requirement.
fn apply_equals(
    doc: &mut DocumentMut,
    key: &str,
    want: u64,
    attribution: &[Provenance],
    findings: &mut Vec<Finding>,
) {
    let current = doc.get(key).and_then(Item::as_integer);
    let want_i64 = i64::try_from(want).ok();
    if current == want_i64 {
        return;
    }
    findings.push(Finding::Mismatch {
        path: key.into(),
        current: current.map(|n| n.to_string()),
        expected: want.to_string(),
        severity: Severity::Error,
        attribution: attribution.to_vec(),
    });
    if let Some(n) = want_i64 {
        doc[key] = value(n);
    }
}

/// Apply a single scalar `AtMost` requirement.
fn apply_at_most(
    doc: &mut DocumentMut,
    key: &str,
    ceiling: u64,
    attribution: &[Provenance],
    findings: &mut Vec<Finding>,
) {
    let current = doc.get(key).and_then(Item::as_integer);
    let ceiling_i64 = i64::try_from(ceiling).unwrap_or(i64::MAX);
    if current.is_some_and(|c| c <= ceiling_i64) {
        return;
    }
    findings.push(Finding::Mismatch {
        path: key.into(),
        current: current.map(|n| n.to_string()),
        expected: format!("at most {ceiling}"),
        severity: Severity::Error,
        attribution: attribution.to_vec(),
    });
    doc[key] = value(ceiling_i64);
}

/// Apply a single scalar `AtLeast` requirement.
fn apply_at_least(
    doc: &mut DocumentMut,
    key: &str,
    floor: u64,
    attribution: &[Provenance],
    findings: &mut Vec<Finding>,
) {
    let current = doc.get(key).and_then(Item::as_integer);
    let floor_i64 = i64::try_from(floor).unwrap_or(0);
    if current.is_some_and(|c| c >= floor_i64) {
        return;
    }
    findings.push(Finding::Mismatch {
        path: key.into(),
        current: current.map(|n| n.to_string()),
        expected: format!("at least {floor}"),
        severity: Severity::Error,
        attribution: attribution.to_vec(),
    });
    doc[key] = value(floor_i64);
}
