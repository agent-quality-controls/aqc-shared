//! Reconciliation for clippy.toml numeric threshold keys.

use std::collections::BTreeMap;

use aqc_file_engine_core::{Finding, Provenance, ResolvedRequirement, Severity};
use toml_edit::{DocumentMut, Item, value};

use crate::requirement::NumericAssertion;

pub(crate) fn apply(
    doc: &mut DocumentMut,
    merged_by_key: &BTreeMap<String, ResolvedRequirement<NumericAssertion, NumericAssertion>>,
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
    resolved: &ResolvedRequirement<NumericAssertion, NumericAssertion>,
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

fn assertion_fails(current: Option<&Item>, assertion: &NumericAssertion) -> bool {
    let current_int = current.and_then(Item::as_integer);
    match assertion {
        NumericAssertion::Equals(want, _) => current_int != i64::try_from(*want).ok(),
        NumericAssertion::AtMost(want, _) => {
            let ceiling = i64::try_from(*want).unwrap_or(i64::MAX);
            !current_int.is_some_and(|value| value <= ceiling)
        }
        NumericAssertion::AtLeast(want, _) => {
            let floor = i64::try_from(*want).unwrap_or(0);
            !current_int.is_some_and(|value| value >= floor)
        }
        NumericAssertion::Range(min, max, _) => {
            let floor = i64::try_from(*min).unwrap_or(0);
            let ceiling = i64::try_from(*max).unwrap_or(i64::MAX);
            !current_int.is_some_and(|value| value >= floor && value <= ceiling)
        }
        NumericAssertion::Present(_) => current_int.is_none(),
        NumericAssertion::Absent(_) => current.is_some(),
    }
}

fn apply_one(
    doc: &mut DocumentMut,
    key: &str,
    assertion: &NumericAssertion,
    attribution: &[Provenance],
    findings: &mut Vec<Finding>,
) {
    match assertion {
        NumericAssertion::Equals(want, message) => {
            apply_equals(doc, key, *want, message, attribution, findings);
        }
        NumericAssertion::AtMost(want, message) => {
            apply_at_most(doc, key, *want, message, attribution, findings);
        }
        NumericAssertion::AtLeast(want, message) => {
            apply_at_least(doc, key, *want, message, attribution, findings);
        }
        NumericAssertion::Range(min, max, message) => {
            apply_range(doc, key, *min, *max, message, attribution, findings);
        }
        NumericAssertion::Present(message) => {
            apply_present(doc, key, message, attribution, findings)
        }
        NumericAssertion::Absent(message) => apply_absent(doc, key, message, attribution, findings),
    }
}

fn apply_present(
    doc: &DocumentMut,
    key: &str,
    message: &str,
    attribution: &[Provenance],
    findings: &mut Vec<Finding>,
) {
    if doc.get(key).and_then(Item::as_integer).is_some() {
        return;
    }
    findings.push(Finding::Mismatch {
        key: key.to_owned(),
        current: None,
        expected: "any integer (Present)".into(),
        message: message.to_owned(),
        severity: Severity::Error,
        attribution: attribution.to_vec(),
    });
}

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
        key: key.to_owned(),
        current: doc
            .get(key)
            .and_then(Item::as_integer)
            .map(|n| n.to_string()),
        expected: "absent".into(),
        message: message.to_owned(),
        severity: Severity::Error,
        attribution: attribution.to_vec(),
    });
    let _ = doc.as_table_mut().remove(key);
}

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
        key: key.to_owned(),
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
        key: key.to_owned(),
        current: current.map(|n| n.to_string()),
        expected: format!("at most {ceiling}"),
        message: message.to_owned(),
        severity: Severity::Error,
        attribution: attribution.to_vec(),
    });
    doc[key] = value(ceiling_i64);
}

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
        key: key.to_owned(),
        current: current.map(|n| n.to_string()),
        expected: format!("at least {floor}"),
        message: message.to_owned(),
        severity: Severity::Error,
        attribution: attribution.to_vec(),
    });
    doc[key] = value(floor_i64);
}

fn apply_range(
    doc: &mut DocumentMut,
    key: &str,
    floor: u64,
    ceiling: u64,
    message: &str,
    attribution: &[Provenance],
    findings: &mut Vec<Finding>,
) {
    let current = doc.get(key).and_then(Item::as_integer);
    let floor_i64 = i64::try_from(floor).unwrap_or(0);
    let ceiling_i64 = i64::try_from(ceiling).unwrap_or(i64::MAX);
    if current.is_some_and(|c| c >= floor_i64 && c <= ceiling_i64) {
        return;
    }
    findings.push(Finding::Mismatch {
        key: key.to_owned(),
        current: current.map(|n| n.to_string()),
        expected: format!("between {floor} and {ceiling}"),
        message: message.to_owned(),
        severity: Severity::Error,
        attribution: attribution.to_vec(),
    });
    let replacement = current.map_or(floor_i64, |c| c.clamp(floor_i64, ceiling_i64));
    doc[key] = value(replacement);
}
