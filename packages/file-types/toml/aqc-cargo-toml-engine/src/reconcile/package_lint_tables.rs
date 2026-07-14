//! Reconcile identities of dynamic `[lints.<tool>]` tables.

use aqc_file_engine_core::{Finding, KeyedItem, Provenance, ResolvedItemRequirements, Severity};
use aqc_toml_engine_core::{ensure_table, table_ref};
use toml_edit::{DocumentMut, Item};

/// Reconcile required, forbidden, and exact local lint-table identities.
pub(crate) fn apply(
    doc: &mut DocumentMut,
    requirements: &ResolvedItemRequirements<KeyedItem<()>>,
    findings: &mut Vec<Finding>,
) {
    let exact_items = requirements.exact.as_ref().map(|exact| &exact.items);
    for (tool, entry) in requirements.required.iter().chain(
        exact_items
            .into_iter()
            .flat_map(|items| items.iter())
            .filter(|(tool, _)| !requirements.required.contains_key(*tool)),
    ) {
        let present =
            table_ref(doc, "lints").is_some_and(|table| table.get(tool).is_some_and(is_lint_table));
        if present {
            continue;
        }
        findings.push(Finding::Mismatch {
            key: format!("[lints.{tool}]"),
            selector: None,
            current: None,
            expected: "table present".to_owned(),
            message: item_message(&entry.collected),
            severity: Severity::Error,
            attribution: item_attribution(&entry.collected),
        });
        ensure_table(doc, "lints")[tool] = Item::Table(toml_edit::Table::new());
    }

    let exact_identities = requirements.exact.as_ref().map(|exact| &exact.identities);
    let existing = table_ref(doc, "lints")
        .map(|table| {
            table
                .iter()
                .filter(|(_, item)| is_lint_table(item))
                .map(|(key, _)| key.to_owned())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    for tool in existing {
        if let Some(entry) = requirements.forbidden.get(&tool) {
            remove_table(doc, &tool);
            findings.push(Finding::Mismatch {
                key: format!("[lints.{tool}]"),
                selector: None,
                current: Some("table present".to_owned()),
                expected: "absent".to_owned(),
                message: entry
                    .collected
                    .first()
                    .map(|(_, message)| message.clone())
                    .unwrap_or_default(),
                severity: Severity::Error,
                attribution: entry
                    .collected
                    .iter()
                    .map(|(prov, _)| prov.clone())
                    .collect(),
            });
        } else if exact_identities.is_some_and(|allowed| !allowed.contains(&tool)) {
            let Some(exact) = requirements.exact.as_ref() else {
                continue;
            };
            remove_table(doc, &tool);
            findings.push(Finding::Mismatch {
                key: format!("[lints.{tool}]"),
                selector: None,
                current: Some("table present".to_owned()),
                expected: "absent (exact collection)".to_owned(),
                message: exact
                    .collected
                    .first()
                    .map(|(_, (_, message))| message.clone())
                    .unwrap_or_default(),
                severity: Severity::Error,
                attribution: exact
                    .collected
                    .iter()
                    .map(|(prov, _)| prov.clone())
                    .collect(),
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

/// Return all contributors to a resolved lint-table identity.
fn item_attribution(collected: &[(Provenance, (KeyedItem<()>, String))]) -> Vec<Provenance> {
    collected.iter().map(|(prov, _)| prov.clone()).collect()
}
