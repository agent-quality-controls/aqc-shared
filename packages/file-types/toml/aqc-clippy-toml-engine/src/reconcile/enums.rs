//! Reconciliation for clippy.toml string-valued (enum-style) settings.

#![cfg_attr(
    not(test),
    expect(
        clippy::missing_docs_in_private_items,
        reason = "Private setting reconciliation helpers are internal reconciliation steps."
    )
)]
#![expect(
    clippy::type_complexity,
    reason = "Private setting reconciliation helpers carry repeated resolved requirement shapes."
)]

use std::collections::{BTreeMap, BTreeSet};

use aqc_file_engine_core::{Finding, Provenance, ResolvedRequirement, Severity};
use toml_edit::{DocumentMut, Item, value};

use crate::requirement::StringAssertion;

/// Apply every string-setting requirement.
pub(crate) fn apply(
    doc: &mut DocumentMut,
    merged_by_key: &BTreeMap<String, ResolvedRequirement<StringAssertion, StringAssertion>>,
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
    resolved: &ResolvedRequirement<StringAssertion, StringAssertion>,
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

fn assertion_fails(current: Option<&Item>, assertion: &StringAssertion) -> bool {
    let current_str = current.and_then(Item::as_str);
    match assertion {
        StringAssertion::Equals(want, _) => current_str != Some(want.as_str()),
        StringAssertion::OneOf(allowed, _) => {
            !current_str.is_some_and(|value| allowed.contains(value))
        }
        StringAssertion::Present(_) => current_str.is_none(),
        StringAssertion::Absent(_) => current.is_some(),
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
