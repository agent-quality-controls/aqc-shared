//! Reconcile identities of dynamic `[lints.<tool>]` tables.

use aqc_file_engine_core::{
    FileItemRequirement, Finding, KeyedItem, Provenance, ResolvedItemRequirements, Severity,
    item_presence_difference,
};
use aqc_toml_engine_core::{ensure_table, table_ref};
use toml_edit::{DocumentMut, Item};

/// Reconcile required, forbidden, and exact local lint-table identities.
pub(crate) fn apply(
    doc: &mut DocumentMut,
    requirements: &ResolvedItemRequirements<KeyedItem<()>>,
    findings: &mut Vec<Finding>,
) {
    let current = table_ref(doc, "lints")
        .map(|table| {
            table
                .iter()
                .filter(|(_, item)| is_lint_table(item))
                .map(|(key, _)| key.to_owned())
                .collect()
        })
        .unwrap_or_default();
    let difference = item_presence_difference(&current, requirements);

    for (tool, entry) in difference.missing {
        findings.push(Finding::Mismatch {
            key: format!("[lints.{tool}]"),
            selector: None,
            current: None,
            expected: "table present".to_owned(),
            message: item_message(&entry.collected),
            severity: Severity::Error,
            attribution: entry.attribution(),
        });
        ensure_table(doc, "lints")[tool] = Item::Table(toml_edit::Table::new());
    }

    for (tool, entry) in difference.forbidden {
        remove_table(doc, tool);
        findings.push(Finding::Mismatch {
            key: format!("[lints.{tool}]"),
            selector: None,
            current: Some("table present".to_owned()),
            expected: "absent".to_owned(),
            message: entry
                .collected
                .first()
                .map_or_else(String::new, |(_, message)| message.clone()),
            severity: Severity::Error,
            attribution: entry.attribution(),
        });
    }
    if let Some(membership) = requirements.membership() {
        for tool in difference.unexpected {
            remove_table(doc, tool);
            findings.push(Finding::Mismatch {
                key: format!("[lints.{tool}]"),
                selector: None,
                current: Some("table present".to_owned()),
                expected: if membership.is_exact() {
                    "absent (exact collection)"
                } else {
                    "absent (not allowed)"
                }
                .to_owned(),
                message: membership
                    .message_for_rejected(|item| item.merge_identity() == *tool)
                    .to_owned(),
                severity: Severity::Error,
                attribution: membership
                    .attribution_for_rejected(|item| item.merge_identity() == *tool),
            });
        }
    }
}

/// Cargo accepts package-local lint groups as standard or inline TOML tables.
fn is_lint_table(item: &Item) -> bool {
    item.is_table_like() || item.as_inline_table().is_some()
}

/// Remove one local lint table while preserving the surrounding `[lints]` table.
fn remove_table(doc: &mut DocumentMut, tool: &str) {
    if let Some(table) = doc.get_mut("lints").and_then(Item::as_table_like_mut) {
        let _ = table.remove(tool);
    }
}

/// Return the first requirement message for a resolved lint-table identity.
fn item_message(collected: &[(Provenance, (KeyedItem<()>, String))]) -> String {
    collected
        .first()
        .map(|(_, (_, message))| message.clone())
        .unwrap_or_default()
}
