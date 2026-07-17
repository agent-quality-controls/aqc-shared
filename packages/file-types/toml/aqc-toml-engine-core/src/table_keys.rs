//! Reconciliation for explicit TOML table-key membership.

use std::collections::BTreeSet;

use aqc_file_engine_core::{
    FileItemRequirement, Finding, KeyedItem, ResolvedItemRequirements, Severity,
    item_presence_difference,
};
use toml_edit::TableLike;

/// Remove and report present keys forbidden by explicit or exact membership.
pub fn remove_rejected_table_keys(
    table: &mut dyn TableLike,
    display_key: &str,
    requirements: &ResolvedItemRequirements<KeyedItem<()>>,
    findings: &mut Vec<Finding>,
) {
    let current_keys = table
        .iter()
        .map(|(key, _)| key.to_owned())
        .collect::<BTreeSet<_>>();
    let difference = item_presence_difference(&current_keys, requirements);

    for (key, resolved) in difference.forbidden {
        let current_value = table.get(key).map(ToString::to_string);
        let _ = table.remove(key);
        findings.push(Finding::Mismatch {
            key: child_key(display_key, key),
            selector: None,
            current: current_value,
            expected: "absent".to_owned(),
            message: resolved
                .collected
                .first()
                .map_or_else(String::new, |(_, message)| message.clone()),
            severity: Severity::Error,
            attribution: resolved.attribution(),
        });
    }
    if let Some(membership) = requirements.membership() {
        for key in difference.unexpected {
            let current_value = table.get(key).map(ToString::to_string);
            let _ = table.remove(key);
            findings.push(Finding::Mismatch {
                key: child_key(display_key, key),
                selector: None,
                current: current_value,
                expected: if membership.is_exact() {
                    "absent (exact keys)"
                } else {
                    "absent (not allowed)"
                }
                .to_owned(),
                message: membership
                    .message_for_rejected(|item| item.merge_identity() == *key)
                    .to_owned(),
                severity: Severity::Error,
                attribution: membership
                    .attribution_for_rejected(|item| item.merge_identity() == *key),
            });
        }
    }
}

/// Report keys still missing after child-value reconciliation has run.
pub fn report_missing_table_keys(
    table: &dyn TableLike,
    display_key: &str,
    requirements: &ResolvedItemRequirements<KeyedItem<()>>,
    findings: &mut Vec<Finding>,
) {
    let current_keys = table
        .iter()
        .map(|(key, _)| key.to_owned())
        .collect::<BTreeSet<_>>();
    let difference = item_presence_difference(&current_keys, requirements);
    for (key, resolved) in difference.missing {
        findings.push(Finding::UnwritableRequiredKey {
            key: child_key(display_key, key),
            expected: "present table key".to_owned(),
            attribution: resolved.attribution(),
        });
    }
}

fn child_key(display_key: &str, key: &str) -> String {
    if display_key.is_empty() {
        key.to_owned()
    } else {
        format!("{display_key}.{key}")
    }
}
