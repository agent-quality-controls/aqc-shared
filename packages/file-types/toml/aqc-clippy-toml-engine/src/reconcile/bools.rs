//! Reconciliation for clippy.toml boolean settings.

use std::collections::BTreeMap;

use aqc_file_engine_core::{Finding, MergedAssertion, Provenance, Severity};
use toml_edit::{DocumentMut, Item, value};

use crate::reconcile::util::all_provenances;
use crate::requirement::BoolAssertion;

/// Apply every boolean-setting contribution.
#[expect(
    clippy::type_complexity,
    reason = "BTreeMap<String, MergedAssertion<...>> is the natural section input shape"
)]
pub(crate) fn apply(
    doc: &mut DocumentMut,
    merged_by_key: &BTreeMap<String, MergedAssertion<BoolAssertion>>,
    findings: &mut Vec<Finding>,
) {
    for (key, merged) in merged_by_key {
        let attribution = all_provenances(merged);
        for (_, assertion) in &merged.contributions {
            apply_one(doc, key, assertion, &attribution, findings);
        }
    }
}

/// Apply a single `BoolAssertion` against a setting.
fn apply_one(
    doc: &mut DocumentMut,
    key: &str,
    assertion: &BoolAssertion,
    attribution: &[Provenance],
    findings: &mut Vec<Finding>,
) {
    match assertion {
        BoolAssertion::Equals(want, message) => {
            apply_equals(doc, key, *want, message, attribution, findings);
        }
        BoolAssertion::Present(message) => apply_present(doc, key, message, attribution, findings),
        BoolAssertion::Absent(message) => apply_absent(doc, key, message, attribution, findings),
    }
}

/// `key == want`.
fn apply_equals(
    doc: &mut DocumentMut,
    key: &str,
    want: bool,
    message: &str,
    attribution: &[Provenance],
    findings: &mut Vec<Finding>,
) {
    let current = doc.get(key).and_then(Item::as_bool);
    if current == Some(want) {
        return;
    }
    findings.push(Finding::Mismatch {
        path: key.into(),
        current: current.map(|b| b.to_string()),
        expected: want.to_string(),
        message: message.to_owned(),
        severity: Severity::Error,
        attribution: attribution.to_vec(),
    });
    doc[key] = value(want);
}

/// `key` must be set with any boolean value.
fn apply_present(
    doc: &DocumentMut,
    key: &str,
    message: &str,
    attribution: &[Provenance],
    findings: &mut Vec<Finding>,
) {
    if doc.get(key).and_then(Item::as_bool).is_some() {
        return;
    }
    findings.push(Finding::Mismatch {
        path: key.into(),
        current: None,
        expected: "any bool (Present)".into(),
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
        path: key.into(),
        current: doc.get(key).and_then(Item::as_bool).map(|b| b.to_string()),
        expected: "absent".into(),
        message: message.to_owned(),
        severity: Severity::Error,
        attribution: attribution.to_vec(),
    });
    let _ = doc.as_table_mut().remove(key);
}
