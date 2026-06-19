//! Reconciliation for clippy.toml's `msrv` key.

#![cfg_attr(
    not(test),
    expect(
        clippy::missing_docs_in_private_items,
        reason = "Private msrv reconciliation helpers are internal steps."
    )
)]

use std::collections::BTreeSet;

use aqc_file_engine_core::{
    DottedVersion, Finding, Provenance, ResolvedRequirement, ScalarAssertion, Severity,
};
use toml_edit::{DocumentMut, Item, value};

/// Apply the resolved `msrv` requirement to the document.
pub(crate) fn apply(
    doc: &mut DocumentMut,
    merged: &ResolvedRequirement<ScalarAssertion<DottedVersion>, ScalarAssertion<DottedVersion>>,
    findings: &mut Vec<Finding>,
) {
    let current = doc
        .get("msrv")
        .and_then(Item::as_str)
        .map(ToOwned::to_owned);
    let attribution = attribution_for(current.as_deref(), merged);

    apply_one(
        doc,
        current.as_deref(),
        &merged.merged,
        &attribution,
        findings,
    );
}

fn attribution_for(
    current: Option<&str>,
    resolved: &ResolvedRequirement<ScalarAssertion<DottedVersion>, ScalarAssertion<DottedVersion>>,
) -> Vec<Provenance> {
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

fn assertion_fails(current: Option<&str>, assertion: &ScalarAssertion<DottedVersion>) -> bool {
    match assertion {
        ScalarAssertion::Equals(want, _) => current != Some(want.as_str()),
        ScalarAssertion::AtLeast(min, _) => {
            !current.is_some_and(|value| DottedVersion::new(value) >= min.clone())
        }
        ScalarAssertion::AtMost(max, _) => {
            !current.is_some_and(|value| DottedVersion::new(value) <= max.clone())
        }
        ScalarAssertion::Range(min, max, _) => !current.is_some_and(|value| {
            let value = DottedVersion::new(value);
            value >= min.clone() && value <= max.clone()
        }),
        ScalarAssertion::OneOf(allowed, _) => {
            !current.is_some_and(|value| allowed.contains(&DottedVersion::new(value)))
        }
        ScalarAssertion::Present(_) => current.is_none(),
        ScalarAssertion::Absent(_) => current.is_some(),
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
        ScalarAssertion::Equals(want, message) => {
            apply_equals(doc, current, want.as_str(), message, attribution, findings);
        }
        ScalarAssertion::AtLeast(min, message) => {
            apply_at_least(doc, current, min, message, attribution, findings);
        }
        ScalarAssertion::AtMost(max, message) => {
            apply_at_most(doc, current, max, message, attribution, findings);
        }
        ScalarAssertion::Range(min, max, message) => {
            apply_range(doc, current, min, max, message, attribution, findings);
        }
        ScalarAssertion::OneOf(allowed, message) => {
            apply_one_of(current, allowed, message, attribution, findings);
        }
        ScalarAssertion::Present(message) => apply_present(current, message, attribution, findings),
        ScalarAssertion::Absent(message) => {
            apply_absent(doc, current, message, attribution, findings);
        }
    }
}

/// `msrv == want`.
fn apply_equals(
    doc: &mut DocumentMut,
    current: Option<&str>,
    want: &str,
    message: &str,
    attribution: &[Provenance],
    findings: &mut Vec<Finding>,
) {
    if current == Some(want) {
        return;
    }
    findings.push(Finding::Mismatch {
        key: "msrv".into(),
        current: current.map(ToOwned::to_owned),
        expected: want.to_owned(),
        message: message.to_owned(),
        severity: Severity::Error,
        attribution: attribution.to_vec(),
    });
    doc["msrv"] = value(want.to_owned());
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

/// `msrv ∈ allowed`.
fn apply_one_of(
    current: Option<&str>,
    allowed: &BTreeSet<DottedVersion>,
    message: &str,
    attribution: &[Provenance],
    findings: &mut Vec<Finding>,
) {
    if current.is_some_and(|c| allowed.contains(&DottedVersion::new(c))) {
        return;
    }
    let allowed = allowed
        .iter()
        .map(DottedVersion::as_str)
        .collect::<Vec<_>>();
    findings.push(Finding::Mismatch {
        key: "msrv".into(),
        current: current.map(ToOwned::to_owned),
        expected: format!("one of {allowed:?}"),
        message: message.to_owned(),
        severity: Severity::Error,
        attribution: attribution.to_vec(),
    });
}

/// `msrv` is set (any value).
fn apply_present(
    current: Option<&str>,
    message: &str,
    attribution: &[Provenance],
    findings: &mut Vec<Finding>,
) {
    if current.is_some() {
        return;
    }
    findings.push(Finding::Mismatch {
        key: "msrv".into(),
        current: None,
        expected: "any value (Present)".into(),
        message: message.to_owned(),
        severity: Severity::Error,
        attribution: attribution.to_vec(),
    });
}

/// `msrv` is not set.
fn apply_absent(
    doc: &mut DocumentMut,
    current: Option<&str>,
    message: &str,
    attribution: &[Provenance],
    findings: &mut Vec<Finding>,
) {
    if current.is_none() {
        return;
    }
    findings.push(Finding::Mismatch {
        key: "msrv".into(),
        current: current.map(ToOwned::to_owned),
        expected: "absent".into(),
        message: message.to_owned(),
        severity: Severity::Error,
        attribution: attribution.to_vec(),
    });
    let _ = doc.as_table_mut().remove("msrv");
}
