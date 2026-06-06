//! Reconcile `[lints.<tool>]` / `[workspace.lints.<tool>]` tables.
//!
//! Each entry on disk can take two forms (both valid cargo TOML):
//!
//! ```toml
//! unwrap_used = "deny"                              # bare string
//! all = { level = "deny", priority = -1 }           # inline table
//! ```
//!
//! For group lints the inline-table form with `priority = -1` is load-bearing.
//! Policy intent is the assertion's optional `priority` slot: `Some(i)` writes
//! inline-table form, `None` writes bare string. The engine reads both.
//!
//! Lazy: an `Excludes`-only requirement against a missing table creates no
//! table (no write, no finding). Tables are fetched mutably only on a write.

use std::collections::{BTreeMap, BTreeSet};

use aqc_file_engine_core::{Finding, MergedAssertion, Provenance, Severity};
use toml_edit::{DocumentMut, InlineTable, Item, Table, Value, value};

use crate::reconcile::util::{all_provenances, ensure_nested, ensure_table, table_ref};
use crate::requirement::LintLevelsAssertion;

/// Where a lints root lives: top-level `[lints]` or nested `[workspace.lints]`.
#[derive(Clone, Copy)]
pub(crate) enum LintRoot {
    /// `[lints]`.
    Package,
    /// `[workspace.lints]`.
    Workspace,
}

impl LintRoot {
    /// The finding-path prefix (`lints` / `workspace.lints`).
    const fn prefix(self) -> &'static str {
        match self {
            Self::Package => "lints",
            Self::Workspace => "workspace.lints",
        }
    }
}

/// Apply every `[lints.<tool>]` (or `[workspace.lints.<tool>]`) contribution.
#[expect(
    clippy::type_complexity,
    reason = "BTreeMap<String, MergedAssertion<...>> is the natural section input shape"
)]
pub(crate) fn apply(
    doc: &mut DocumentMut,
    root: LintRoot,
    merged_by_tool: &BTreeMap<String, MergedAssertion<LintLevelsAssertion>>,
    findings: &mut Vec<Finding>,
) {
    for (tool, merged) in merged_by_tool {
        apply_tool(doc, root, tool, merged, findings);
    }
}

/// Read-only view of the tool's table, if it exists.
fn tool_ref<'a>(doc: &'a DocumentMut, root: LintRoot, tool: &str) -> Option<&'a Table> {
    let lints_root = match root {
        LintRoot::Package => table_ref(doc, "lints"),
        LintRoot::Workspace => {
            table_ref(doc, "workspace").and_then(|ws| ws.get("lints").and_then(Item::as_table))
        }
    }?;
    lints_root.get(tool).and_then(Item::as_table)
}

/// Mutable view of the tool's table, creating roots lazily (call only on write).
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

/// Mutable view of an existing tool table (removals only; no creation).
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

/// Apply contributions for one tool's lint table.
fn apply_tool(
    doc: &mut DocumentMut,
    root: LintRoot,
    tool: &str,
    merged: &MergedAssertion<LintLevelsAssertion>,
    findings: &mut Vec<Finding>,
) {
    for (_, assertion) in &merged.contributions {
        match assertion {
            LintLevelsAssertion::Contains(map) | LintLevelsAssertion::IsExactly(map) => {
                apply_contains(doc, root, tool, merged, map, findings);
            }
            LintLevelsAssertion::Excludes(map) => {
                apply_excludes(doc, root, tool, merged, map, findings);
            }
        }
    }
    if let Some(exact) = is_exactly_only(merged) {
        apply_exact_extras(doc, root, tool, merged, &exact, findings);
    }
}

/// `Contains` / `IsExactly` per-key: write each in the form its priority implies.
#[expect(
    clippy::type_complexity,
    reason = "BTreeMap<String, (level, priority, message)> mirrors the assertion's value shape."
)]
fn apply_contains(
    doc: &mut DocumentMut,
    root: LintRoot,
    tool: &str,
    merged: &MergedAssertion<LintLevelsAssertion>,
    map: &BTreeMap<String, (String, Option<i64>, String)>,
    findings: &mut Vec<Finding>,
) {
    for (lint, (level, priority, message)) in map {
        let on_disk = tool_ref(doc, root, tool).and_then(|t| read_entry(t, lint));
        if matches_policy(on_disk.as_ref(), level, *priority) {
            continue;
        }
        findings.push(Finding::Mismatch {
            path: format!("[{}.{tool}].{lint}", root.prefix()),
            current: on_disk.map(|e| display_entry(&e)),
            expected: display_expected(level, *priority),
            message: message.clone(),
            severity: Severity::Error,
            attribution: contributors_for_lint(merged, lint),
        });
        tool_mut(doc, root, tool)[lint] = write_entry(level, *priority);
    }
}

/// `Excludes`: remove any of the named keys (vacuous when the table is absent).
fn apply_excludes(
    doc: &mut DocumentMut,
    root: LintRoot,
    tool: &str,
    merged: &MergedAssertion<LintLevelsAssertion>,
    map: &BTreeMap<String, String>,
    findings: &mut Vec<Finding>,
) {
    for (lint, message) in map {
        let Some(entry) = tool_ref(doc, root, tool).and_then(|t| read_entry(t, lint)) else {
            continue;
        };
        findings.push(Finding::Mismatch {
            path: format!("[{}.{tool}].{lint}", root.prefix()),
            current: Some(display_entry(&entry)),
            expected: "absent".to_owned(),
            message: message.clone(),
            severity: Severity::Error,
            attribution: contributors_for_lint(merged, lint),
        });
        if let Some(t) = tool_mut_existing(doc, root, tool) {
            let _ = t.remove(lint);
        }
    }
}

/// Drop on-disk keys not in any `IsExactly` contribution.
#[expect(
    clippy::type_complexity,
    reason = "BTreeMap<String, (level, priority, message)> mirrors the assertion's value shape."
)]
fn apply_exact_extras(
    doc: &mut DocumentMut,
    root: LintRoot,
    tool: &str,
    merged: &MergedAssertion<LintLevelsAssertion>,
    exact: &BTreeMap<String, (String, Option<i64>, String)>,
    findings: &mut Vec<Finding>,
) {
    let Some(table) = tool_ref(doc, root, tool) else {
        return;
    };
    let on_disk: BTreeSet<String> = table.iter().map(|(k, _)| k.to_owned()).collect();
    let allowed: BTreeSet<String> = exact.keys().cloned().collect();
    let extras: Vec<String> = on_disk.difference(&allowed).cloned().collect();
    for extra in &extras {
        let current = tool_ref(doc, root, tool)
            .and_then(|t| read_entry(t, extra))
            .map(|e| display_entry(&e));
        findings.push(Finding::Mismatch {
            path: format!("[{}.{tool}].{extra}", root.prefix()),
            current,
            expected: "absent (IsExactly)".to_owned(),
            message: String::new(),
            severity: Severity::Error,
            attribution: all_provenances(merged),
        });
        if let Some(t) = tool_mut_existing(doc, root, tool) {
            let _ = t.remove(extra);
        }
    }
}

/// One on-disk lint-table entry, typed.
struct LintEntry {
    /// Lint level (`"deny"`, `"warn"`, `"allow"`, `"forbid"`).
    level: String,
    /// Priority field for inline-table form; `None` for bare-string form.
    priority: Option<i64>,
}

/// Parse the on-disk entry for `key`, accepting bare string or inline table.
fn read_entry(table: &Table, key: &str) -> Option<LintEntry> {
    let item = table.get(key)?;
    if let Some(s) = item.as_str() {
        return Some(LintEntry {
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
        return Some(LintEntry { level, priority });
    }
    None
}

/// True if the on-disk entry already matches what the policy asks for.
fn matches_policy(on_disk: Option<&LintEntry>, level: &str, priority: Option<i64>) -> bool {
    on_disk.is_some_and(|e| e.level == level && e.priority == priority)
}

/// Build the item to write, picking bare string or inline table by priority.
fn write_entry(level: &str, priority: Option<i64>) -> Item {
    priority.map_or_else(
        || value(level.to_owned()),
        |p| {
            let mut t = InlineTable::new();
            let _ = t.insert("level", Value::from(level));
            let _ = t.insert("priority", Value::from(p));
            Item::Value(Value::InlineTable(t))
        },
    )
}

/// Render an on-disk entry for the finding's `current` field.
fn display_entry(e: &LintEntry) -> String {
    e.priority.map_or_else(
        || e.level.clone(),
        |p| format!("{{ level = \"{level}\", priority = {p} }}", level = e.level),
    )
}

/// Render the expected entry for the finding's `expected` field.
fn display_expected(level: &str, priority: Option<i64>) -> String {
    priority.map_or_else(
        || level.to_owned(),
        |p| format!("{{ level = \"{level}\", priority = {p} }}"),
    )
}

/// Provenances of contributions that mention `lint`.
fn contributors_for_lint(
    merged: &MergedAssertion<LintLevelsAssertion>,
    lint: &str,
) -> Vec<Provenance> {
    let mut out = Vec::new();
    for (provenance, assertion) in &merged.contributions {
        let mentions = match assertion {
            LintLevelsAssertion::Contains(map) | LintLevelsAssertion::IsExactly(map) => {
                map.contains_key(lint)
            }
            LintLevelsAssertion::Excludes(map) => map.contains_key(lint),
        };
        if mentions {
            out.push(provenance.clone());
        }
    }
    out
}

/// Union of allowed keys if every contribution is `IsExactly`; else `None`.
#[expect(
    clippy::type_complexity,
    reason = "BTreeMap<String, (level, priority, message)> mirrors the assertion's `IsExactly` value shape."
)]
fn is_exactly_only(
    merged: &MergedAssertion<LintLevelsAssertion>,
) -> Option<BTreeMap<String, (String, Option<i64>, String)>> {
    let mut combined: BTreeMap<String, (String, Option<i64>, String)> = BTreeMap::new();
    for (_, assertion) in &merged.contributions {
        match assertion {
            LintLevelsAssertion::IsExactly(map) => {
                for (k, v) in map {
                    let _ = combined.insert(k.clone(), v.clone());
                }
            }
            LintLevelsAssertion::Contains(_) | LintLevelsAssertion::Excludes(_) => return None,
        }
    }
    if combined.is_empty() {
        None
    } else {
        Some(combined)
    }
}
