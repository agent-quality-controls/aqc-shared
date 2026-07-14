//! Reconciliation for clippy.toml numeric threshold keys.

#![cfg_attr(
    not(test),
    expect(
        clippy::missing_docs_in_private_items,
        reason = "Private threshold reconciliation helpers are internal reconciliation steps."
    )
)]
#![allow(
    clippy::type_complexity,
    reason = "Private threshold reconciliation helpers carry repeated resolved requirement shapes."
)]

use std::collections::BTreeMap;

use aqc_file_engine_core::{
    ConfigScalar, Finding, Provenance, ResolvedRequirement, ScalarAssertion, Severity,
};
use aqc_toml_engine_core::{apply_scalar_assertion, scalar_assertion_fails};
use toml_edit::{DocumentMut, Item, value};

pub(crate) fn apply(
    doc: &mut DocumentMut,
    merged_by_key: &BTreeMap<
        String,
        ResolvedRequirement<ScalarAssertion<u64>, ScalarAssertion<u64>>,
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
    resolved: &ResolvedRequirement<ScalarAssertion<u64>, ScalarAssertion<u64>>,
) -> Vec<Provenance> {
    let current = doc.get(key);
    let filtered = resolved
        .collected
        .iter()
        .filter(|(_, assertion)| assertion_fails(current, assertion))
        .map(|(prov, _)| prov.clone())
        .collect::<Vec<_>>();
    if filtered.is_empty() {
        resolved.attribution()
    } else {
        filtered
    }
}

fn assertion_fails(current: Option<&Item>, assertion: &ScalarAssertion<u64>) -> bool {
    if let Some(assertion) = numeric_assertion_to_config(assertion) {
        return scalar_assertion_fails(current, &assertion);
    }
    let current_int = current.and_then(Item::as_integer);
    match assertion {
        ScalarAssertion::AtMost(want, _) => {
            let ceiling = i64::try_from(*want).unwrap_or(i64::MAX);
            current_int.is_none_or(|value| value > ceiling)
        }
        ScalarAssertion::AtLeast(want, _) => {
            let floor = i64::try_from(*want).unwrap_or(0);
            current_int.is_none_or(|value| value < floor)
        }
        ScalarAssertion::Range(min, max, _) => {
            let floor = i64::try_from(*min).unwrap_or(0);
            let ceiling = i64::try_from(*max).unwrap_or(i64::MAX);
            !current_int.is_some_and(|value| value >= floor && value <= ceiling)
        }
        ScalarAssertion::Equals(want, _) => current_int != i64::try_from(*want).ok(),
        ScalarAssertion::OneOf(allowed, _) => !current_int
            .is_some_and(|value| u64::try_from(value).is_ok_and(|value| allowed.contains(&value))),
        ScalarAssertion::Present(_) => current_int.is_none(),
        ScalarAssertion::Absent(_) => current.is_some(),
    }
}

fn apply_one(
    doc: &mut DocumentMut,
    key: &str,
    assertion: &ScalarAssertion<u64>,
    attribution: &[Provenance],
    findings: &mut Vec<Finding>,
) {
    match assertion {
        ScalarAssertion::AtMost(want, message) => {
            apply_at_most(doc, key, *want, message, attribution, findings);
        }
        ScalarAssertion::AtLeast(want, message) => {
            apply_at_least(doc, key, *want, message, attribution, findings);
        }
        ScalarAssertion::Range(min, max, message) => {
            apply_range(doc, key, *min, *max, message, attribution, findings);
        }
        ScalarAssertion::Equals(..)
        | ScalarAssertion::OneOf(..)
        | ScalarAssertion::Present(_)
        | ScalarAssertion::Absent(_) => {
            if let Some(assertion) = numeric_assertion_to_config(assertion) {
                apply_scalar_assertion(doc, key, &assertion, attribution, findings);
            } else if let ScalarAssertion::Equals(want, message) = assertion {
                apply_unrepresentable_equals(doc, key, *want, message, attribution, findings);
            }
        }
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
        selector: None,
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
        selector: None,
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
        selector: None,
        current: current.map(|n| n.to_string()),
        expected: format!("between {floor} and {ceiling}"),
        message: message.to_owned(),
        severity: Severity::Error,
        attribution: attribution.to_vec(),
    });
    let replacement = current.map_or(floor_i64, |c| c.clamp(floor_i64, ceiling_i64));
    doc[key] = value(replacement);
}

fn apply_unrepresentable_equals(
    doc: &DocumentMut,
    key: &str,
    want: u64,
    message: &str,
    attribution: &[Provenance],
    findings: &mut Vec<Finding>,
) {
    let current = doc.get(key).and_then(Item::as_integer);
    findings.push(Finding::Mismatch {
        key: key.to_owned(),
        selector: None,
        current: current.map(|n| n.to_string()),
        expected: want.to_string(),
        message: message.to_owned(),
        severity: Severity::Error,
        attribution: attribution.to_vec(),
    });
}

fn numeric_assertion_to_config(
    assertion: &ScalarAssertion<u64>,
) -> Option<ScalarAssertion<ConfigScalar>> {
    match assertion {
        ScalarAssertion::Equals(value, msg) => Some(ScalarAssertion::Equals(
            ConfigScalar::Int(i64::try_from(*value).ok()?),
            msg.clone(),
        )),
        ScalarAssertion::OneOf(values, msg) => Some(ScalarAssertion::OneOf(
            values
                .iter()
                .map(|value| i64::try_from(*value).map(ConfigScalar::Int))
                .collect::<Result<_, _>>()
                .ok()?,
            msg.clone(),
        )),
        ScalarAssertion::Present(msg) => Some(ScalarAssertion::Present(msg.clone())),
        ScalarAssertion::Absent(msg) => Some(ScalarAssertion::Absent(msg.clone())),
        ScalarAssertion::AtLeast(..) | ScalarAssertion::AtMost(..) | ScalarAssertion::Range(..) => {
            None
        }
    }
}
