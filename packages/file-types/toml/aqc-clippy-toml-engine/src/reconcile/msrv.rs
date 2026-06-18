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
    Finding, Provenance, ResolvedRequirement, Severity, parse_version_tuple,
};
use toml_edit::{DocumentMut, Item, value};

use crate::requirement::MsrvAssertion;

/// Apply the resolved `msrv` requirement to the document.
pub(crate) fn apply(
    doc: &mut DocumentMut,
    merged: &ResolvedRequirement<MsrvAssertion, MsrvAssertion>,
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
    resolved: &ResolvedRequirement<MsrvAssertion, MsrvAssertion>,
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

fn assertion_fails(current: Option<&str>, assertion: &MsrvAssertion) -> bool {
    match assertion {
        MsrvAssertion::Equals(want, _) => current != Some(want.as_str()),
        MsrvAssertion::AtLeast(min, _) => !current.is_some_and(|value| ge_version(value, min)),
        MsrvAssertion::OneOf(allowed, _) => !current.is_some_and(|value| allowed.contains(value)),
        MsrvAssertion::Present(_) => current.is_none(),
        MsrvAssertion::Absent(_) => current.is_some(),
    }
}

/// Apply a single `MsrvAssertion` against the current on-disk value.
fn apply_one(
    doc: &mut DocumentMut,
    current: Option<&str>,
    assertion: &MsrvAssertion,
    attribution: &[Provenance],
    findings: &mut Vec<Finding>,
) {
    match assertion {
        MsrvAssertion::Equals(want, message) => {
            apply_equals(doc, current, want, message, attribution, findings);
        }
        MsrvAssertion::AtLeast(min, message) => {
            apply_at_least(doc, current, min, message, attribution, findings);
        }
        MsrvAssertion::OneOf(allowed, message) => {
            apply_one_of(current, allowed, message, attribution, findings);
        }
        MsrvAssertion::Present(message) => apply_present(current, message, attribution, findings),
        MsrvAssertion::Absent(message) => {
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
    min: &str,
    message: &str,
    attribution: &[Provenance],
    findings: &mut Vec<Finding>,
) {
    if current.is_some_and(|c| ge_version(c, min)) {
        return;
    }
    findings.push(Finding::Mismatch {
        key: "msrv".into(),
        current: current.map(ToOwned::to_owned),
        expected: format!("at least {min}"),
        message: message.to_owned(),
        severity: Severity::Error,
        attribution: attribution.to_vec(),
    });
    doc["msrv"] = value(min.to_owned());
}

/// `msrv ∈ allowed`.
fn apply_one_of(
    current: Option<&str>,
    allowed: &BTreeSet<String>,
    message: &str,
    attribution: &[Provenance],
    findings: &mut Vec<Finding>,
) {
    if current.is_some_and(|c| allowed.iter().any(|a| a == c)) {
        return;
    }
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

/// Compare semver-ish dotted version strings.
fn ge_version(a: &str, b: &str) -> bool {
    parse_version_tuple(a) >= parse_version_tuple(b)
}
