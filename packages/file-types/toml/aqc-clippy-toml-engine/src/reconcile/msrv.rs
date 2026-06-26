//! Reconciliation for clippy.toml's `msrv` key.

#![cfg_attr(
    not(test),
    expect(
        clippy::missing_docs_in_private_items,
        reason = "Private msrv reconciliation helpers are internal steps."
    )
)]

use aqc_file_engine_core::{
    ConfigScalar, DottedVersion, Finding, Provenance, ResolvedRequirement, ScalarAssertion,
    Severity,
};
use aqc_toml_engine_core::{apply_scalar_assertion, scalar_assertion_fails};
use toml_edit::{DocumentMut, Item, value};

/// Resolved clippy `msrv` scalar assertion.
type ResolvedMsrv =
    ResolvedRequirement<ScalarAssertion<DottedVersion>, ScalarAssertion<DottedVersion>>;

/// Apply the resolved `msrv` requirement to the document.
pub(crate) fn apply(doc: &mut DocumentMut, merged: &ResolvedMsrv, findings: &mut Vec<Finding>) {
    let current_item = doc.get("msrv");
    let current = current_item.and_then(Item::as_str).map(ToOwned::to_owned);
    let attribution = attribution_for(current_item, merged);

    apply_one(
        doc,
        current.as_deref(),
        &merged.merged,
        &attribution,
        findings,
    );
}

fn attribution_for(current: Option<&Item>, resolved: &ResolvedMsrv) -> Vec<Provenance> {
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

fn assertion_fails(current: Option<&Item>, assertion: &ScalarAssertion<DottedVersion>) -> bool {
    if let Some(assertion) = msrv_assertion_to_config(assertion) {
        return scalar_assertion_fails(current, &assertion);
    }
    let current = current.and_then(Item::as_str);
    match assertion {
        ScalarAssertion::AtLeast(min, _) => {
            current.is_none_or(|value| DottedVersion::new(value) < min.clone())
        }
        ScalarAssertion::AtMost(max, _) => {
            current.is_none_or(|value| DottedVersion::new(value) > max.clone())
        }
        ScalarAssertion::Range(min, max, _) => !current.is_some_and(|value| {
            let value = DottedVersion::new(value);
            value >= min.clone() && value <= max.clone()
        }),
        ScalarAssertion::Equals(..)
        | ScalarAssertion::OneOf(..)
        | ScalarAssertion::Present(_)
        | ScalarAssertion::Absent(_) => false,
    }
}

/// Apply a single scalar assertion against the current on-disk `msrv` value.
fn apply_one(
    doc: &mut DocumentMut,
    current: Option<&str>,
    assertion: &ScalarAssertion<DottedVersion>,
    attribution: &[Provenance],
    findings: &mut Vec<Finding>,
) {
    match assertion {
        ScalarAssertion::AtLeast(min, message) => {
            apply_at_least(doc, current, min, message, attribution, findings);
        }
        ScalarAssertion::AtMost(max, message) => {
            apply_at_most(doc, current, max, message, attribution, findings);
        }
        ScalarAssertion::Range(min, max, message) => {
            apply_range(doc, current, min, max, message, attribution, findings);
        }
        ScalarAssertion::Equals(..)
        | ScalarAssertion::OneOf(..)
        | ScalarAssertion::Present(_)
        | ScalarAssertion::Absent(_) => {
            if let Some(assertion) = msrv_assertion_to_config(assertion) {
                apply_scalar_assertion(doc, "msrv", &assertion, attribution, findings);
            }
        }
    }
}

/// `msrv >= min`.
fn apply_at_least(
    doc: &mut DocumentMut,
    current: Option<&str>,
    min: &DottedVersion,
    message: &str,
    attribution: &[Provenance],
    findings: &mut Vec<Finding>,
) {
    if current.is_some_and(|c| DottedVersion::new(c) >= min.clone()) {
        return;
    }
    findings.push(Finding::Mismatch {
        key: "msrv".into(),
        current: current.map(ToOwned::to_owned),
        expected: format!("at least {}", min.as_str()),
        message: message.to_owned(),
        severity: Severity::Error,
        attribution: attribution.to_vec(),
    });
    doc["msrv"] = value(min.as_str().to_owned());
}

fn apply_at_most(
    doc: &mut DocumentMut,
    current: Option<&str>,
    max: &DottedVersion,
    message: &str,
    attribution: &[Provenance],
    findings: &mut Vec<Finding>,
) {
    if current.is_some_and(|c| DottedVersion::new(c) <= max.clone()) {
        return;
    }
    findings.push(Finding::Mismatch {
        key: "msrv".into(),
        current: current.map(ToOwned::to_owned),
        expected: format!("at most {}", max.as_str()),
        message: message.to_owned(),
        severity: Severity::Error,
        attribution: attribution.to_vec(),
    });
    doc["msrv"] = value(max.as_str().to_owned());
}

fn apply_range(
    doc: &mut DocumentMut,
    current: Option<&str>,
    min: &DottedVersion,
    max: &DottedVersion,
    message: &str,
    attribution: &[Provenance],
    findings: &mut Vec<Finding>,
) {
    let replacement = current.map_or_else(
        || min.as_str(),
        |value| {
            let version = DottedVersion::new(value);
            if version < min.clone() {
                min.as_str()
            } else if version > max.clone() {
                max.as_str()
            } else {
                value
            }
        },
    );
    if current == Some(replacement) {
        return;
    }
    findings.push(Finding::Mismatch {
        key: "msrv".into(),
        current: current.map(ToOwned::to_owned),
        expected: format!("between {} and {}", min.as_str(), max.as_str()),
        message: message.to_owned(),
        severity: Severity::Error,
        attribution: attribution.to_vec(),
    });
    doc["msrv"] = value(replacement.to_owned());
}

fn msrv_assertion_to_config(
    assertion: &ScalarAssertion<DottedVersion>,
) -> Option<ScalarAssertion<ConfigScalar>> {
    match assertion {
        ScalarAssertion::Equals(value, msg) => Some(ScalarAssertion::Equals(
            ConfigScalar::Str(value.as_str().to_owned()),
            msg.clone(),
        )),
        ScalarAssertion::OneOf(values, msg) => Some(ScalarAssertion::OneOf(
            values
                .iter()
                .map(|value| ConfigScalar::Str(value.as_str().to_owned()))
                .collect(),
            msg.clone(),
        )),
        ScalarAssertion::Present(msg) => Some(ScalarAssertion::Present(msg.clone())),
        ScalarAssertion::Absent(msg) => Some(ScalarAssertion::Absent(msg.clone())),
        ScalarAssertion::AtLeast(..) | ScalarAssertion::AtMost(..) | ScalarAssertion::Range(..) => {
            None
        }
    }
}
