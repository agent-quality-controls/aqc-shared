//! Reconcile `[profile.<name>]` tables.

use std::collections::BTreeMap;

use aqc_file_engine_core::{Finding, MergedAssertion, Provenance, Severity};
use toml_edit::{Item, Table, Value};

use crate::reconcile::util::{
    all_provenances, get_or_create_nested_table_mut, get_or_create_table_mut,
};
use crate::requirement::{ProfileAssertion, ProfileFieldAssertion};

/// Apply every `[profile.<name>]` contribution.
#[expect(
    clippy::type_complexity,
    reason = "BTreeMap<String, MergedAssertion<...>> is the natural section input shape"
)]
pub(crate) fn apply(
    doc: &mut toml_edit::DocumentMut,
    merged_by_profile: &BTreeMap<String, MergedAssertion<ProfileAssertion>>,
    findings: &mut Vec<Finding>,
) {
    if merged_by_profile.is_empty() {
        return;
    }
    let profile_root = get_or_create_table_mut(doc, "profile");
    for (profile, merged) in merged_by_profile {
        apply_profile(profile_root, profile, merged, findings);
    }
}

/// Apply contributions for one profile.
fn apply_profile(
    profile_root: &mut Table,
    profile: &str,
    merged: &MergedAssertion<ProfileAssertion>,
    findings: &mut Vec<Finding>,
) {
    let attribution = all_provenances(merged);
    let table = get_or_create_nested_table_mut(profile_root, profile);
    for (_, assertion) in &merged.contributions {
        match assertion {
            ProfileAssertion::Fields(field_map) => {
                for (field, field_assertion) in field_map {
                    apply_field(
                        table,
                        profile,
                        field,
                        field_assertion,
                        &attribution,
                        findings,
                    );
                }
            }
        }
    }
}

/// Apply one `ProfileFieldAssertion` to a profile field.
fn apply_field(
    table: &mut Table,
    profile: &str,
    field: &str,
    assertion: &ProfileFieldAssertion,
    attribution: &[Provenance],
    findings: &mut Vec<Finding>,
) {
    match assertion {
        ProfileFieldAssertion::Equals(want) => {
            apply_equals(table, profile, field, want, attribution, findings);
        }
        ProfileFieldAssertion::OneOf(allowed) => {
            apply_one_of(table, profile, field, allowed, attribution, findings);
        }
        ProfileFieldAssertion::Present => {
            apply_present(table, profile, field, attribution, findings);
        }
        ProfileFieldAssertion::Absent => {
            apply_absent(table, profile, field, attribution, findings);
        }
    }
}

/// `field == want`.
fn apply_equals(
    table: &mut Table,
    profile: &str,
    field: &str,
    want: &Value,
    attribution: &[Provenance],
    findings: &mut Vec<Finding>,
) {
    let current = table.get(field).and_then(Item::as_value);
    if current.is_some_and(|c| values_equal(c, want)) {
        return;
    }
    findings.push(Finding::Mismatch {
        path: format!("[profile.{profile}].{field}"),
        current: current.map(ToString::to_string),
        expected: want.to_string(),
        severity: Severity::Error,
        attribution: attribution.to_vec(),
    });
    table[field] = Item::Value(want.clone());
}

/// `field ∈ allowed`.
fn apply_one_of(
    table: &Table,
    profile: &str,
    field: &str,
    allowed: &[Value],
    attribution: &[Provenance],
    findings: &mut Vec<Finding>,
) {
    let current = table.get(field).and_then(Item::as_value);
    if current.is_some_and(|c| allowed.iter().any(|a| values_equal(c, a))) {
        return;
    }
    findings.push(Finding::Mismatch {
        path: format!("[profile.{profile}].{field}"),
        current: current.map(ToString::to_string),
        expected: format!("one of {allowed:?}"),
        severity: Severity::Error,
        attribution: attribution.to_vec(),
    });
}

/// `field` must be set.
fn apply_present(
    table: &Table,
    profile: &str,
    field: &str,
    attribution: &[Provenance],
    findings: &mut Vec<Finding>,
) {
    if table.contains_key(field) {
        return;
    }
    findings.push(Finding::Mismatch {
        path: format!("[profile.{profile}].{field}"),
        current: None,
        expected: "any value (Present)".into(),
        severity: Severity::Error,
        attribution: attribution.to_vec(),
    });
}

/// `field` must not be set.
fn apply_absent(
    table: &mut Table,
    profile: &str,
    field: &str,
    attribution: &[Provenance],
    findings: &mut Vec<Finding>,
) {
    if !table.contains_key(field) {
        return;
    }
    findings.push(Finding::Mismatch {
        path: format!("[profile.{profile}].{field}"),
        current: table
            .get(field)
            .and_then(Item::as_value)
            .map(ToString::to_string),
        expected: "absent".into(),
        severity: Severity::Error,
        attribution: attribution.to_vec(),
    });
    let _ = table.remove(field);
}

/// Compare two `toml_edit::Value`s by display form. The `toml_edit` crate
/// does not derive `PartialEq`, so we compare textually after rendering.
fn values_equal(a: &Value, b: &Value) -> bool {
    a.to_string().trim() == b.to_string().trim()
}
