//! Reconcile `[features]`.

#![allow(
    clippy::type_complexity,
    reason = "Feature reconciliation consumes resolved item requirement shapes."
)]
//!
//! Lazy: a forbid-only requirement against a missing `[features]` table
//! writes nothing. The table is fetched mutably only on a write.

use std::collections::BTreeSet;

use aqc_file_engine_core::{Finding, KeyedItem, Provenance, ResolvedItemRequirements, Severity};
use aqc_toml_engine_core::{
    ensure_table, table_list_values, table_list_values_optional, table_ref, write_table_list,
};
use toml_edit::{DocumentMut, Item};

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
    for entry in merged.forbidden.values() {
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
        apply_forbidden(doc, feature, &msg, &attribution, findings);
    }
    if let Some(exact) = &merged.exact {
        let attribution = exact
            .collected
            .iter()
            .map(|(prov, _)| prov.clone())
            .collect::<Vec<_>>();
        let allowed = exact.identities.clone();
        let message = exact
            .collected
            .first()
            .map(|(_, (_, message))| message.as_str())
            .unwrap_or_default();
        apply_exact_extras(doc, &allowed, message, &attribution, findings);
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
    let current = table_ref(doc, "features").and_then(|t| table_list_values_optional(t, feature));
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
    let members = want.members.iter().cloned().collect::<Vec<_>>();
    write_table_list(ensure_table(doc, "features"), feature, &members);
}

/// Each named feature must be absent (vacuous when the table is missing).
fn apply_forbidden(
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
        table_ref(doc, "features").map_or_else(Vec::new, |t| table_list_values(t, feature));
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

/// Drop features not present in the exact collection.
fn apply_exact_extras(
    doc: &mut DocumentMut,
    allowed: &BTreeSet<String>,
    message: &str,
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
            table_ref(doc, "features").map_or_else(Vec::new, |t| table_list_values(t, extra));
        findings.push(Finding::Mismatch {
            key: format!("[features].{extra}"),
            current: Some(format!("{current:?}")),
            expected: "absent (exact collection)".to_owned(),
            message: message.to_owned(),
            severity: Severity::Error,
            attribution: attribution.to_vec(),
        });
        if let Some(t) = doc.get_mut("features").and_then(Item::as_table_mut) {
            let _ = t.remove(extra);
        }
    }
}
