//! Reconcile `[package].<field>` (and `[workspace.package].<field>` via
//! `apply_into_table`).

use std::collections::{BTreeMap, BTreeSet};

use aqc_file_engine_core::{Finding, MergedAssertion, Provenance, Severity, parse_version_tuple};
use toml_edit::{Array, Item, Table, Value, value};

use crate::reconcile::util::{all_provenances, get_or_create_table_mut};
use crate::requirement::PackageFieldAssertion;

/// Apply every `[package].<field>` contribution.
#[expect(
    clippy::type_complexity,
    reason = "BTreeMap<String, MergedAssertion<...>> is the natural section input shape"
)]
pub(crate) fn apply(
    doc: &mut toml_edit::DocumentMut,
    merged_by_field: &BTreeMap<String, MergedAssertion<PackageFieldAssertion>>,
    findings: &mut Vec<Finding>,
) {
    if merged_by_field.is_empty() {
        return;
    }
    let package = get_or_create_table_mut(doc, "package");
    for (field, merged) in merged_by_field {
        apply_into_table(package, "package", field, merged, findings);
    }
}

/// Apply contributions for one field into the given table.
///
/// Reused by `workspace_package_fields` to target `[workspace.package]`.
pub(crate) fn apply_into_table(
    table: &mut Table,
    section_prefix: &str,
    field: &str,
    merged: &MergedAssertion<PackageFieldAssertion>,
    findings: &mut Vec<Finding>,
) {
    let attribution = all_provenances(merged);
    for (_, assertion) in &merged.contributions {
        apply_one(
            table,
            section_prefix,
            field,
            assertion,
            &attribution,
            findings,
        );
    }
}

/// Apply a single `PackageFieldAssertion` to one field.
fn apply_one(
    table: &mut Table,
    section_prefix: &str,
    field: &str,
    assertion: &PackageFieldAssertion,
    attribution: &[Provenance],
    findings: &mut Vec<Finding>,
) {
    match assertion {
        PackageFieldAssertion::Equals(want) => {
            apply_equals(table, section_prefix, field, want, attribution, findings);
        }
        PackageFieldAssertion::AtLeast(min) => {
            apply_at_least(table, section_prefix, field, min, attribution, findings);
        }
        PackageFieldAssertion::OneOf(allowed) => {
            apply_one_of(table, section_prefix, field, allowed, attribution, findings);
        }
        PackageFieldAssertion::ListContains(items) => {
            apply_list_contains(table, section_prefix, field, items, attribution, findings);
        }
        PackageFieldAssertion::ListIsExactly(items) => {
            apply_list_is_exactly(table, section_prefix, field, items, attribution, findings);
        }
        PackageFieldAssertion::Present => {
            apply_present(table, section_prefix, field, attribution, findings);
        }
        PackageFieldAssertion::Absent => {
            apply_absent(table, section_prefix, field, attribution, findings);
        }
    }
}

/// Enforce a scalar `field == want`.
fn apply_equals(
    table: &mut Table,
    section_prefix: &str,
    field: &str,
    want: &str,
    attribution: &[Provenance],
    findings: &mut Vec<Finding>,
) {
    let current = current_str(table, field);
    if current.as_deref() == Some(want) {
        return;
    }
    findings.push(Finding::Mismatch {
        path: format!("[{section_prefix}].{field}"),
        current,
        expected: want.to_owned(),
        message: String::new(),
        severity: Severity::Error,
        attribution: attribution.to_vec(),
    });
    table[field] = value(want.to_owned());
}

/// Enforce a scalar `field >= min` (dotted version comparison).
fn apply_at_least(
    table: &mut Table,
    section_prefix: &str,
    field: &str,
    min: &str,
    attribution: &[Provenance],
    findings: &mut Vec<Finding>,
) {
    let current = current_str(table, field);
    if current.as_deref().is_some_and(|c| ge_version(c, min)) {
        return;
    }
    findings.push(Finding::Mismatch {
        path: format!("[{section_prefix}].{field}"),
        current,
        expected: format!("at least {min}"),
        message: String::new(),
        severity: Severity::Error,
        attribution: attribution.to_vec(),
    });
    table[field] = value(min.to_owned());
}

/// Enforce `field ∈ allowed`.
fn apply_one_of(
    table: &Table,
    section_prefix: &str,
    field: &str,
    allowed: &BTreeSet<String>,
    attribution: &[Provenance],
    findings: &mut Vec<Finding>,
) {
    let current = current_str(table, field);
    if current.as_deref().is_some_and(|c| allowed.contains(c)) {
        return;
    }
    findings.push(Finding::Mismatch {
        path: format!("[{section_prefix}].{field}"),
        current,
        expected: format!("one of {allowed:?}"),
        message: String::new(),
        severity: Severity::Error,
        attribution: attribution.to_vec(),
    });
}

/// Enforce that the on-disk list contains every requested element.
fn apply_list_contains(
    table: &mut Table,
    section_prefix: &str,
    field: &str,
    items: &[String],
    attribution: &[Provenance],
    findings: &mut Vec<Finding>,
) {
    let on_disk = current_list(table, field);
    let on_disk_set: BTreeSet<&str> = on_disk.iter().map(String::as_str).collect();
    let mut missing = Vec::new();
    for w in items {
        if !on_disk_set.contains(w.as_str()) {
            missing.push(w.clone());
        }
    }
    if missing.is_empty() {
        return;
    }
    findings.push(Finding::Mismatch {
        path: format!("[{section_prefix}].{field}"),
        current: Some(format!("{on_disk:?}")),
        expected: format!("contains {missing:?}"),
        message: String::new(),
        severity: Severity::Error,
        attribution: attribution.to_vec(),
    });
    let mut new_list = on_disk;
    for w in items {
        if !new_list.iter().any(|e| e == w) {
            new_list.push(w.clone());
        }
    }
    write_string_list(table, field, &new_list);
}

/// Enforce that the on-disk list equals exactly the requested set.
fn apply_list_is_exactly(
    table: &mut Table,
    section_prefix: &str,
    field: &str,
    items: &[String],
    attribution: &[Provenance],
    findings: &mut Vec<Finding>,
) {
    let on_disk = current_list(table, field);
    if on_disk == items {
        return;
    }
    findings.push(Finding::Mismatch {
        path: format!("[{section_prefix}].{field}"),
        current: Some(format!("{on_disk:?}")),
        expected: format!("{items:?}"),
        message: String::new(),
        severity: Severity::Error,
        attribution: attribution.to_vec(),
    });
    write_string_list(table, field, items);
}

/// Enforce that `field` is set (any value).
fn apply_present(
    table: &Table,
    section_prefix: &str,
    field: &str,
    attribution: &[Provenance],
    findings: &mut Vec<Finding>,
) {
    if table.contains_key(field) {
        return;
    }
    findings.push(Finding::Mismatch {
        path: format!("[{section_prefix}].{field}"),
        current: None,
        expected: "any value (Present)".into(),
        message: String::new(),
        severity: Severity::Error,
        attribution: attribution.to_vec(),
    });
}

/// Enforce that `field` is unset.
fn apply_absent(
    table: &mut Table,
    section_prefix: &str,
    field: &str,
    attribution: &[Provenance],
    findings: &mut Vec<Finding>,
) {
    if !table.contains_key(field) {
        return;
    }
    findings.push(Finding::Mismatch {
        path: format!("[{section_prefix}].{field}"),
        current: current_str(table, field),
        expected: "absent".into(),
        message: String::new(),
        severity: Severity::Error,
        attribution: attribution.to_vec(),
    });
    let _ = table.remove(field);
}

/// Compare semver-ish dotted version strings (`1.85`, `1.85.0`).
fn ge_version(a: &str, b: &str) -> bool {
    parse_version_tuple(a) >= parse_version_tuple(b)
}

/// Read the current scalar string for `field`, if any.
fn current_str(table: &Table, field: &str) -> Option<String> {
    table
        .get(field)
        .and_then(Item::as_str)
        .map(ToOwned::to_owned)
}

/// Read the current array of strings for `field`. Returns empty vec if absent.
fn current_list(table: &Table, field: &str) -> Vec<String> {
    let Some(arr) = table.get(field).and_then(Item::as_array) else {
        return Vec::new();
    };
    arr.iter()
        .filter_map(|v| v.as_str().map(ToOwned::to_owned))
        .collect()
}

/// Write `field = ["a", "b", ...]` into `table`.
fn write_string_list(table: &mut Table, field: &str, items: &[String]) {
    let mut arr = Array::new();
    for it in items {
        arr.push(Value::from(it.as_str()));
    }
    table[field] = Item::Value(Value::Array(arr));
}
