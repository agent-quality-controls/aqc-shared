//! Reconcile `[features]`.

use std::collections::{BTreeMap, BTreeSet};

use aqc_file_engine_core::{Finding, MergedAssertion, Provenance, Severity};
use toml_edit::{Array, Item, Table, Value};

use crate::reconcile::util::{all_provenances, get_or_create_table_mut};
use crate::requirement::FeatureSetAssertion;

/// Apply the `[features]` contribution (single field on the requirement).
pub(crate) fn apply(
    doc: &mut toml_edit::DocumentMut,
    merged: Option<&MergedAssertion<FeatureSetAssertion>>,
    findings: &mut Vec<Finding>,
) {
    let Some(merged) = merged else { return };
    let attribution = all_provenances(merged);
    let table = get_or_create_table_mut(doc, "features");
    for (_, assertion) in &merged.contributions {
        match assertion {
            FeatureSetAssertion::Contains(map) | FeatureSetAssertion::IsExactly(map) => {
                apply_contains(table, map, &attribution, findings);
            }
            FeatureSetAssertion::Excludes(names) => {
                apply_excludes(table, names, &attribution, findings);
            }
        }
    }
    if let Some(exact) = is_exactly_only(merged) {
        apply_exact_extras(table, &exact, &attribution, findings);
    }
}

/// Each `(feature, impl_list)` must be present and match.
#[expect(
    clippy::type_complexity,
    reason = "BTreeMap<String, BTreeSet<String>> matches the on-disk shape of [features]."
)]
fn apply_contains(
    table: &mut Table,
    map: &BTreeMap<String, BTreeSet<String>>,
    attribution: &[Provenance],
    findings: &mut Vec<Finding>,
) {
    for (feature, want_impls) in map {
        let current = current_list(table, feature);
        let current_set: BTreeSet<String> = current.iter().cloned().collect();
        if current_set == *want_impls {
            continue;
        }
        findings.push(Finding::Mismatch {
            path: format!("[features].{feature}"),
            current: Some(format!("{current:?}")),
            expected: format!("{want_impls:?}"),
            message: String::new(),
            severity: Severity::Error,
            attribution: attribution.to_vec(),
        });
        write_list(table, feature, want_impls);
    }
}

/// Each named feature must be absent.
fn apply_excludes(
    table: &mut Table,
    names: &BTreeSet<String>,
    attribution: &[Provenance],
    findings: &mut Vec<Finding>,
) {
    for feature in names {
        if !table.contains_key(feature) {
            continue;
        }
        findings.push(Finding::Mismatch {
            path: format!("[features].{feature}"),
            current: Some(format!("{:?}", current_list(table, feature))),
            expected: "absent".into(),
            message: String::new(),
            severity: Severity::Error,
            attribution: attribution.to_vec(),
        });
        let _ = table.remove(feature);
    }
}

/// Drop features not present in the `IsExactly` union.
#[expect(
    clippy::type_complexity,
    reason = "BTreeMap<String, BTreeSet<String>> matches the on-disk shape of [features]."
)]
fn apply_exact_extras(
    table: &mut Table,
    allowed: &BTreeMap<String, BTreeSet<String>>,
    attribution: &[Provenance],
    findings: &mut Vec<Finding>,
) {
    let on_disk: BTreeSet<String> = table.iter().map(|(k, _)| k.to_owned()).collect();
    let allowed_keys: BTreeSet<String> = allowed.keys().cloned().collect();
    for extra in on_disk.difference(&allowed_keys) {
        findings.push(Finding::Mismatch {
            path: format!("[features].{extra}"),
            current: Some(format!("{:?}", current_list(table, extra))),
            expected: "absent (IsExactly)".into(),
            message: String::new(),
            severity: Severity::Error,
            attribution: attribution.to_vec(),
        });
        let _ = table.remove(extra);
    }
}

/// Read the current array-of-strings for `feature`.
fn current_list(table: &Table, feature: &str) -> Vec<String> {
    let Some(arr) = table.get(feature).and_then(Item::as_array) else {
        return Vec::new();
    };
    arr.iter()
        .filter_map(|v| v.as_str().map(ToOwned::to_owned))
        .collect()
}

/// Write `feature = ["impl1", "impl2", ...]`.
fn write_list(table: &mut Table, feature: &str, impls: &BTreeSet<String>) {
    let mut arr = Array::new();
    for i in impls {
        arr.push(Value::from(i.as_str()));
    }
    table[feature] = Item::Value(Value::Array(arr));
}

/// If every contribution is `IsExactly`, return the union; else None.
#[expect(
    clippy::type_complexity,
    reason = "BTreeMap<String, BTreeSet<String>> is the natural shape for the returned mapping."
)]
fn is_exactly_only(
    merged: &MergedAssertion<FeatureSetAssertion>,
) -> Option<BTreeMap<String, BTreeSet<String>>> {
    let mut combined: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for (_, assertion) in &merged.contributions {
        match assertion {
            FeatureSetAssertion::IsExactly(map) => {
                for (k, v) in map {
                    let _ = combined.insert(k.clone(), v.clone());
                }
            }
            FeatureSetAssertion::Contains(_) | FeatureSetAssertion::Excludes(_) => return None,
        }
    }
    if combined.is_empty() {
        None
    } else {
        Some(combined)
    }
}
