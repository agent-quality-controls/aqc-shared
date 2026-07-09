//! Reconcile `[lints.<tool>]` and `[workspace.lints.<tool>]` tables.

#![cfg_attr(
    not(test),
    expect(
        clippy::missing_docs_in_private_items,
        reason = "Private lint-table helpers are internal reconciliation steps."
    )
)]
#![allow(
    clippy::too_many_arguments,
    clippy::type_complexity,
    reason = "Lint table reconciliation passes TOML location, policy values, and findings together."
)]

use std::collections::{BTreeMap, BTreeSet};

use aqc_file_engine_core::{Finding, KeyedItem, Provenance, ResolvedItemRequirements, Severity};
use aqc_toml_engine_core::{ensure_nested, ensure_table, table_ref};
use toml_edit::{DocumentMut, InlineTable, Item, Table, TableLike, Value, value};

use crate::requirement::LintSetting as RequiredLintSetting;

#[derive(Clone, Copy)]
pub(crate) enum LintRoot {
    Package,
    Workspace,
}

impl LintRoot {
    const fn prefix(self) -> &'static str {
        match self {
            Self::Package => "lints",
            Self::Workspace => "workspace.lints",
        }
    }
}

pub(crate) fn apply(
    doc: &mut DocumentMut,
    root: LintRoot,
    merged_by_tool: &BTreeMap<String, ResolvedItemRequirements<KeyedItem<RequiredLintSetting>>>,
    findings: &mut Vec<Finding>,
) {
    for (tool, merged) in merged_by_tool {
        apply_tool(doc, root, tool, merged, findings);
    }
}

fn tool_ref<'a>(doc: &'a DocumentMut, root: LintRoot, tool: &str) -> Option<&'a dyn TableLike> {
    let lints_root = match root {
        LintRoot::Package => table_ref(doc, "lints"),
        LintRoot::Workspace => {
            table_ref(doc, "workspace").and_then(|ws| ws.get("lints").and_then(Item::as_table_like))
        }
    }?;
    lints_root.get(tool).and_then(Item::as_table_like)
}

fn tool_mut<'a>(doc: &'a mut DocumentMut, root: LintRoot, tool: &str) -> &'a mut Table {
    let lints_root = match root {
        LintRoot::Package => ensure_table(doc, "lints"),
        LintRoot::Workspace => {
            let ws = ensure_table(doc, "workspace");
            ensure_nested(ws, "lints")
        }
    };
    ensure_nested(lints_root, tool)
}

fn tool_mut_existing<'a>(
    doc: &'a mut DocumentMut,
    root: LintRoot,
    tool: &str,
) -> Option<&'a mut Table> {
    let lints_root = match root {
        LintRoot::Package => doc.get_mut("lints").and_then(Item::as_table_mut),
        LintRoot::Workspace => doc
            .get_mut("workspace")
            .and_then(Item::as_table_mut)
            .and_then(|ws| ws.get_mut("lints").and_then(Item::as_table_mut)),
    }?;
    lints_root.get_mut(tool).and_then(Item::as_table_mut)
}

fn apply_tool(
    doc: &mut DocumentMut,
    root: LintRoot,
    tool: &str,
    merged: &ResolvedItemRequirements<KeyedItem<RequiredLintSetting>>,
    findings: &mut Vec<Finding>,
) {
    for entry in merged.required.values() {
        let lint = &entry.merged.file_key;
        let attribution = entry
            .collected
            .iter()
            .map(|(prov, _)| prov.clone())
            .collect::<Vec<_>>();
        let level = &entry.merged.value.level;
        let priority = entry.merged.value.priority;
        let message = entry
            .collected
            .first()
            .map(|(_, (_, msg))| msg.clone())
            .unwrap_or_default();
        apply_required(
            doc,
            root,
            tool,
            lint,
            level,
            priority,
            &message,
            &attribution,
            findings,
        );
    }
    for entry in merged.forbidden.values() {
        let lint = &entry.merged.file_key;
        let attribution = entry
            .collected
            .iter()
            .map(|(prov, _)| prov.clone())
            .collect::<Vec<_>>();
        let message = entry
            .collected
            .first()
            .map(|(_, msg)| msg.clone())
            .unwrap_or_default();
        apply_forbidden(doc, root, tool, lint, &message, &attribution, findings);
    }
    if !merged.closed_by.is_empty() {
        let allowed = merged
            .required
            .values()
            .map(|entry| entry.merged.file_key.clone())
            .collect();
        let attribution = merged
            .closed_by
            .iter()
            .map(|(prov, _)| prov.clone())
            .collect::<Vec<_>>();
        apply_closed_extras(doc, root, tool, &allowed, &attribution, findings);
    }
}

fn apply_required(
    doc: &mut DocumentMut,
    root: LintRoot,
    tool: &str,
    lint: &str,
    level: &str,
    priority: Option<i64>,
    message: &str,
    attribution: &[Provenance],
    findings: &mut Vec<Finding>,
) {
    let on_disk = tool_ref(doc, root, tool).and_then(|t| read_entry(t, lint));
    if matches_policy(on_disk.as_ref(), level, priority) {
        return;
    }
    findings.push(Finding::Mismatch {
        key: format!("[{}.{tool}].{lint}", root.prefix()),
        current: on_disk.map(|entry| display_entry(&entry)),
        expected: display_expected(level, priority),
        message: message.to_owned(),
        severity: Severity::Error,
        attribution: attribution.to_vec(),
    });
    tool_mut(doc, root, tool)[lint] = write_entry(level, priority);
}

fn apply_forbidden(
    doc: &mut DocumentMut,
    root: LintRoot,
    tool: &str,
    lint: &str,
    message: &str,
    attribution: &[Provenance],
    findings: &mut Vec<Finding>,
) {
    let Some(current) = tool_ref(doc, root, tool).and_then(|t| {
        t.get(lint).map(|item| {
            read_entry(t, lint).map_or_else(
                || item.to_string().trim().to_owned(),
                |entry| display_entry(&entry),
            )
        })
    }) else {
        return;
    };
    findings.push(Finding::Mismatch {
        key: format!("[{}.{tool}].{lint}", root.prefix()),
        current: Some(current),
        expected: "absent".to_owned(),
        message: message.to_owned(),
        severity: Severity::Error,
        attribution: attribution.to_vec(),
    });
    if let Some(table) = tool_mut_existing(doc, root, tool) {
        let _ = table.remove(lint);
    }
}

fn apply_closed_extras(
    doc: &mut DocumentMut,
    root: LintRoot,
    tool: &str,
    allowed: &BTreeSet<String>,
    attribution: &[Provenance],
    findings: &mut Vec<Finding>,
) {
    let Some(table) = tool_ref(doc, root, tool) else {
        return;
    };
    let on_disk = table
        .iter()
        .map(|(key, _)| key.to_owned())
        .collect::<BTreeSet<_>>();
    let extras = on_disk.difference(allowed).cloned().collect::<Vec<_>>();
    for extra in &extras {
        let current = tool_ref(doc, root, tool)
            .and_then(|current_table| read_entry(current_table, extra))
            .map(|entry| display_entry(&entry));
        findings.push(Finding::Mismatch {
            key: format!("[{}.{tool}].{extra}", root.prefix()),
            current,
            expected: "absent (closed table)".to_owned(),
            message: String::new(),
            severity: Severity::Error,
            attribution: attribution.to_vec(),
        });
        if let Some(current_table) = tool_mut_existing(doc, root, tool) {
            let _ = current_table.remove(extra);
        }
    }
}

struct DiskLintSetting {
    level: String,
    priority: Option<i64>,
}

fn read_entry(table: &dyn TableLike, key: &str) -> Option<DiskLintSetting> {
    let item = table.get(key)?;
    if let Some(s) = item.as_str() {
        return Some(DiskLintSetting {
            level: s.to_owned(),
            priority: None,
        });
    }
    if let Some(t) = item.as_inline_table() {
        let level = t
            .get("level")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned)?;
        let priority = t.get("priority").and_then(Value::as_integer);
        return Some(DiskLintSetting { level, priority });
    }
    None
}

fn matches_policy(on_disk: Option<&DiskLintSetting>, level: &str, priority: Option<i64>) -> bool {
    on_disk.is_some_and(|entry| entry.level == level && entry.priority == priority)
}

fn write_entry(level: &str, priority: Option<i64>) -> Item {
    priority.map_or_else(
        || value(level.to_owned()),
        |p| {
            let mut table = InlineTable::new();
            let _ = table.insert("level", Value::from(level));
            let _ = table.insert("priority", Value::from(p));
            Item::Value(Value::InlineTable(table))
        },
    )
}

fn display_entry(entry: &DiskLintSetting) -> String {
    entry.priority.map_or_else(
        || entry.level.clone(),
        |priority| {
            format!(
                "{{ level = \"{level}\", priority = {priority} }}",
                level = entry.level
            )
        },
    )
}

fn display_expected(level: &str, priority: Option<i64>) -> String {
    priority.map_or_else(
        || level.to_owned(),
        |p| format!("{{ level = \"{level}\", priority = {p} }}"),
    )
}
