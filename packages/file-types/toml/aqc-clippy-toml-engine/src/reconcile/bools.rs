//! Reconciliation for clippy.toml boolean settings.

use std::collections::BTreeMap;

use aqc_file_engine_core::{Finding, Provenance, ResolvedRequirement, Severity};
use toml_edit::{DocumentMut, Item, value};

use crate::requirement::BoolAssertion;

/// Apply every boolean-setting requirement.
pub(crate) fn apply(
    doc: &mut DocumentMut,
    merged_by_key: &BTreeMap<String, ResolvedRequirement<BoolAssertion, BoolAssertion>>,
    findings: &mut Vec<Finding>,
) {
    for (key, merged) in merged_by_key {
        let attribution = attribution_for(doc, key, merged);
        apply_one(doc, key, &merged.merged, &attribution, findings);
    }
}

fn attribution_for(
    doc: &DocumentMut,
    key: &str,
    resolved: &ResolvedRequirement<BoolAssertion, BoolAssertion>,
) -> Vec<Provenance> {
    let current = doc.get(key);
    let filtered = resolved
        .collected
        .iter()
        .filter(|(_, assertion)| assertion_fails(current, assertion))
        .map(|(prov, _)| prov.clone())
        .collect::<Vec<_>>();
    if filtered.is_empty() {
        resolved
            .collected
            .iter()
            .map(|(prov, _)| prov.clone())
            .collect()
    } else {
        filtered
    }
}

fn assertion_fails(current: Option<&Item>, assertion: &BoolAssertion) -> bool {
    let current_bool = current.and_then(Item::as_bool);
    match assertion {
        BoolAssertion::Equals(want, _) => current_bool != Some(*want),
        BoolAssertion::Present(_) => current_bool.is_none(),
        BoolAssertion::Absent(_) => current.is_some(),
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
        key: key.into(),
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
        key: key.into(),
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
        key: key.into(),
        current: doc.get(key).and_then(Item::as_bool).map(|b| b.to_string()),
        expected: "absent".into(),
        message: message.to_owned(),
        severity: Severity::Error,
        attribution: attribution.to_vec(),
    });
    let _ = doc.as_table_mut().remove(key);
}
