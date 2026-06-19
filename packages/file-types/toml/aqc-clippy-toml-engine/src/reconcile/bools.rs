//! Reconciliation for clippy.toml boolean settings.

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

use std::collections::BTreeMap;

use aqc_file_engine_core::{Finding, Provenance, ResolvedRequirement, ScalarAssertion, Severity};
use toml_edit::{DocumentMut, Item, value};

/// Apply every boolean-setting requirement.
pub(crate) fn apply(
    doc: &mut DocumentMut,
    merged_by_key: &BTreeMap<
        String,
        ResolvedRequirement<ScalarAssertion<bool>, ScalarAssertion<bool>>,
    >,
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
    resolved: &ResolvedRequirement<ScalarAssertion<bool>, ScalarAssertion<bool>>,
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

fn assertion_fails(current: Option<&Item>, assertion: &ScalarAssertion<bool>) -> bool {
    let current_bool = current.and_then(Item::as_bool);
    match assertion {
        ScalarAssertion::Equals(want, _) => current_bool != Some(*want),
        ScalarAssertion::OneOf(allowed, _) => {
            !current_bool.is_some_and(|value| allowed.contains(&value))
        }
        ScalarAssertion::Present(_) => current_bool.is_none(),
        ScalarAssertion::Absent(_) => current.is_some(),
        ScalarAssertion::AtLeast(..) | ScalarAssertion::AtMost(..) | ScalarAssertion::Range(..) => {
            true
        }
    }
}

/// Apply a single scalar assertion against a boolean setting.
fn apply_one(
    doc: &mut DocumentMut,
    key: &str,
    assertion: &ScalarAssertion<bool>,
    attribution: &[Provenance],
    findings: &mut Vec<Finding>,
) {
    match assertion {
        ScalarAssertion::Equals(want, message) => {
            apply_equals(doc, key, *want, message, attribution, findings);
        }
        ScalarAssertion::OneOf(allowed, message) => {
            apply_one_of(doc, key, allowed, message, attribution, findings);
        }
        ScalarAssertion::Present(message) => {
            apply_present(doc, key, message, attribution, findings)
        }
        ScalarAssertion::Absent(message) => apply_absent(doc, key, message, attribution, findings),
        ScalarAssertion::AtLeast(..) | ScalarAssertion::AtMost(..) | ScalarAssertion::Range(..) => {
        }
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

fn apply_one_of(
    doc: &DocumentMut,
    key: &str,
    allowed: &std::collections::BTreeSet<bool>,
    message: &str,
    attribution: &[Provenance],
    findings: &mut Vec<Finding>,
) {
    let current = doc.get(key).and_then(Item::as_bool);
    if current.is_some_and(|value| allowed.contains(&value)) {
        return;
    }
    findings.push(Finding::Mismatch {
        key: key.into(),
        current: current.map(|b| b.to_string()),
        expected: format!("one of {allowed:?}"),
        message: message.to_owned(),
        severity: Severity::Error,
        attribution: attribution.to_vec(),
    });
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
