//! Reconcile `[features]`.
//!
//! Lazy: a ban-only requirement against a missing `[features]` table
//! writes nothing. The table is fetched mutably only on a write.

use std::collections::BTreeSet;

use aqc_file_engine_core::{Finding, KeyedItem, Provenance, ResolvedItemRequirements, Severity};
use toml_edit::{Array, DocumentMut, Item, Table, Value};

use crate::reconcile::util::{ensure_table, read_string_array, table_ref};
use crate::requirement::FeatureMembers;

/// Apply the `[features]` requirement.
pub(crate) fn apply(
    doc: &mut DocumentMut,
    merged: Option<&ResolvedItemRequirements<KeyedItem<FeatureMembers>>>,
    findings: &mut Vec<Finding>,
) {
    let Some(merged) = merged else { return };
    for entry in merged.required.values() {
        let feature = &entry.merged.file_key;
        let attribution = entry
            .collected
            .iter()
            .map(|(prov, _)| prov.clone())
            .collect::<Vec<_>>();
        let msg = entry
            .collected
            .first()
            .map(|(_, (_, msg))| msg.clone())
            .unwrap_or_default();
        apply_required(
            doc,
            feature,
            &entry.merged.value,
            &msg,
            &attribution,
            findings,
        );
    }
    for entry in merged.banned.values() {
        let feature = &entry.merged.file_key;
        let attribution = entry
            .collected
            .iter()
            .map(|(prov, _)| prov.clone())
            .collect::<Vec<_>>();
        let msg = entry
            .collected
            .first()
            .map(|(_, msg)| msg.clone())
            .unwrap_or_default();
        apply_banned(doc, feature, &msg, &attribution, findings);
    }
    if !merged.closed_by.is_empty() {
        let attribution = merged
            .closed_by
            .iter()
            .map(|(prov, _)| prov.clone())
            .collect::<Vec<_>>();
        let allowed = merged
            .required
            .values()
            .map(|entry| entry.merged.file_key.clone())
            .collect();
        apply_exact_extras(doc, &allowed, &attribution, findings);
    }
}

/// Each `(feature, enable_list)` must be present and equal.
fn apply_required(
    doc: &mut DocumentMut,
    feature: &str,
    want: &FeatureMembers,
    msg: &str,
    attribution: &[Provenance],
    findings: &mut Vec<Finding>,
) {
    let current = table_ref(doc, "features").and_then(|t| read_feature_entry(t, feature));
    if current
        .as_ref()
        .is_some_and(|items| items.iter().cloned().collect::<BTreeSet<_>>() == want.members)
    {
        return;
    }
    findings.push(Finding::Mismatch {
        key: format!("[features].{feature}"),
        current: current.map(|items| format!("{items:?}")),
        expected: format!("{:?}", want.members),
        message: msg.to_owned(),
        severity: Severity::Error,
        attribution: attribution.to_vec(),
    });
    write_list(ensure_table(doc, "features"), feature, want);
}

fn read_feature_entry(table: &Table, feature: &str) -> Option<Vec<String>> {
    let item = table.get(feature)?;
    let arr = item.as_array()?;
    Some(
        arr.iter()
            .filter_map(|value| value.as_str().map(ToOwned::to_owned))
            .collect(),
    )
}

/// Each named feature must be absent (vacuous when the table is missing).
fn apply_banned(
    doc: &mut DocumentMut,
    feature: &str,
    msg: &str,
    attribution: &[Provenance],
    findings: &mut Vec<Finding>,
) {
    if !table_ref(doc, "features").is_some_and(|t| t.contains_key(feature)) {
        return;
    }
    let current =
        table_ref(doc, "features").map_or_else(Vec::new, |t| read_string_array(t, feature));
    findings.push(Finding::Mismatch {
        key: format!("[features].{feature}"),
        current: Some(format!("{current:?}")),
        expected: "absent".to_owned(),
        message: msg.to_owned(),
        severity: Severity::Error,
        attribution: attribution.to_vec(),
    });
    if let Some(t) = doc.get_mut("features").and_then(Item::as_table_mut) {
        let _ = t.remove(feature);
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
            expected: "absent (closed table)".to_owned(),
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
fn write_list(table: &mut Table, feature: &str, value: &FeatureMembers) {
    let mut arr = Array::new();
    for i in &value.members {
        arr.push(Value::from(i.as_str()));
    }
    table[feature] = Item::Value(Value::Array(arr));
}
