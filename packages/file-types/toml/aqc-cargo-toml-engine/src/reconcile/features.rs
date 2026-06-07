//! Reconcile `[features]`.
//!
//! Lazy: an `Excludes`-only requirement against a missing `[features]` table
//! writes nothing. The table is fetched mutably only on a write.

#![expect(
    clippy::type_complexity,
    reason = "Collected assertions are plainly Vec<(Provenance, A)> and per-key maps of them; the shapes are declared openly at every signature instead of hidden behind wrapper types or aliases (taxonomy decision 2026-06-07)."
)]
use std::collections::{BTreeMap, BTreeSet};

use aqc_file_engine_core::{Finding, Provenance, Severity};
use toml_edit::{Array, DocumentMut, Item, Table, Value};

use crate::reconcile::util::{all_provenances, ensure_table, read_string_array, table_ref};
use crate::requirement::FeatureSetAssertion;

/// Apply the `[features]` contribution (single field on the requirement).
pub(crate) fn apply(
    doc: &mut DocumentMut,
    merged: Option<&Vec<(Provenance, FeatureSetAssertion)>>,
    findings: &mut Vec<Finding>,
) {
    let Some(merged) = merged else { return };
    let attribution = all_provenances(merged);
    for (_, assertion) in merged {
        match assertion {
            FeatureSetAssertion::Contains(map) | FeatureSetAssertion::IsExactly(map) => {
                apply_contains(doc, map, &attribution, findings);
            }
            FeatureSetAssertion::Excludes(names) => {
                apply_excludes(doc, names, &attribution, findings);
            }
        }
    }
    if let Some(exact) = is_exactly_only(merged) {
        apply_exact_extras(doc, &exact, &attribution, findings);
    }
}

/// Each `(feature, enable_list)` must be present and equal.
fn apply_contains(
    doc: &mut DocumentMut,
    map: &BTreeMap<String, (BTreeSet<String>, String)>,
    attribution: &[Provenance],
    findings: &mut Vec<Finding>,
) {
    for (feature, (want, msg)) in map {
        let current =
            table_ref(doc, "features").map_or_else(Vec::new, |t| read_string_array(t, feature));
        let current_set: BTreeSet<String> = current.iter().cloned().collect();
        if current_set == *want {
            continue;
        }
        findings.push(Finding::Mismatch {
            key: format!("[features].{feature}"),
            current: Some(format!("{current:?}")),
            expected: format!("{want:?}"),
            message: msg.clone(),
            severity: Severity::Error,
            attribution: attribution.to_vec(),
        });
        write_list(ensure_table(doc, "features"), feature, want);
    }
}

/// Each named feature must be absent (vacuous when the table is missing).
fn apply_excludes(
    doc: &mut DocumentMut,
    names: &BTreeMap<String, String>,
    attribution: &[Provenance],
    findings: &mut Vec<Finding>,
) {
    for (feature, msg) in names {
        if !table_ref(doc, "features").is_some_and(|t| t.contains_key(feature)) {
            continue;
        }
        let current =
            table_ref(doc, "features").map_or_else(Vec::new, |t| read_string_array(t, feature));
        findings.push(Finding::Mismatch {
            key: format!("[features].{feature}"),
            current: Some(format!("{current:?}")),
            expected: "absent".to_owned(),
            message: msg.clone(),
            severity: Severity::Error,
            attribution: attribution.to_vec(),
        });
        if let Some(t) = doc.get_mut("features").and_then(Item::as_table_mut) {
            let _ = t.remove(feature);
        }
    }
}

/// Drop features not present in the `IsExactly` union.
fn apply_exact_extras(
    doc: &mut DocumentMut,
    allowed: &BTreeSet<String>,
    attribution: &[Provenance],
    findings: &mut Vec<Finding>,
) {
    let Some(table) = table_ref(doc, "features") else {
        return;
    };
    let on_disk: BTreeSet<String> = table.iter().map(|(k, _)| k.to_owned()).collect();
    let extras: Vec<String> = on_disk.difference(allowed).cloned().collect();
    for extra in &extras {
        let current =
            table_ref(doc, "features").map_or_else(Vec::new, |t| read_string_array(t, extra));
        findings.push(Finding::Mismatch {
            key: format!("[features].{extra}"),
            current: Some(format!("{current:?}")),
            expected: "absent (IsExactly)".to_owned(),
            message: String::new(),
            severity: Severity::Error,
            attribution: attribution.to_vec(),
        });
        if let Some(t) = doc.get_mut("features").and_then(Item::as_table_mut) {
            let _ = t.remove(extra);
        }
    }
}

/// Write `feature = ["a", "b", ...]`.
fn write_list(table: &mut Table, feature: &str, impls: &BTreeSet<String>) {
    let mut arr = Array::new();
    for i in impls {
        arr.push(Value::from(i.as_str()));
    }
    table[feature] = Item::Value(Value::Array(arr));
}

/// Union of allowed feature names if every contribution is `IsExactly`; else `None`.
fn is_exactly_only(merged: &Vec<(Provenance, FeatureSetAssertion)>) -> Option<BTreeSet<String>> {
    let mut combined: BTreeSet<String> = BTreeSet::new();
    for (_, assertion) in merged {
        match assertion {
            FeatureSetAssertion::IsExactly(map) => combined.extend(map.keys().cloned()),
            FeatureSetAssertion::Contains(_) | FeatureSetAssertion::Excludes(_) => return None,
        }
    }
    if combined.is_empty() {
        None
    } else {
        Some(combined)
    }
}
