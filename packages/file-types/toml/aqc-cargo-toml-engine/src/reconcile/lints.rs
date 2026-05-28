//! Reconcile `[lints.<tool>]` tables.
//!
//! Each entry on disk can take two forms (both valid cargo TOML):
//!
//! ```toml
//! unwrap_used = "deny"                              # bare string
//! all = { level = "deny", priority = -1 }           # inline table
//! ```
//!
//! For group lints (`clippy::all`, `pedantic`, ...) the inline-table form
//! with `priority = -1` is load-bearing: cargo applies lints in priority
//! order, lowest first. Without `-1`, group expansion can clobber
//! per-lint settings.
//!
//! Policy intent is expressed via the assertion's optional `priority`
//! slot. When `Some(i)` the engine writes inline-table form; when `None`
//! it writes bare string. The engine reads both forms regardless.

use std::collections::{BTreeMap, BTreeSet};

use aqc_file_engine_core::{Finding, MergedAssertion, Provenance, Severity};
use toml_edit::{InlineTable, Item, Table, Value, value};

/// Default message when a contribution's entry has none (legacy fallback).
const NO_MESSAGE: &str = "";

use crate::reconcile::util::{
    all_provenances, get_or_create_nested_table_mut, get_or_create_table_mut,
};
use crate::requirement::LintLevelsAssertion;

/// Apply every `[lints.<tool>]` contribution.
#[expect(
    clippy::type_complexity,
    reason = "BTreeMap<String, MergedAssertion<...>> is the natural section input shape"
)]
pub(crate) fn apply(
    doc: &mut toml_edit::DocumentMut,
    merged_by_tool: &BTreeMap<String, MergedAssertion<LintLevelsAssertion>>,
    findings: &mut Vec<Finding>,
) {
    if merged_by_tool.is_empty() {
        return;
    }
    let lints_root = get_or_create_table_mut(doc, "lints");
    for (tool, merged) in merged_by_tool {
        apply_tool(lints_root, "lints", tool, merged, findings);
    }
}

/// Apply contributions for one tool's lint table.
pub(crate) fn apply_tool(
    lints_root: &mut Table,
    section_prefix: &str,
    tool: &str,
    merged: &MergedAssertion<LintLevelsAssertion>,
    findings: &mut Vec<Finding>,
) {
    let tool_table = get_or_create_nested_table_mut(lints_root, tool);
    for (_, assertion) in &merged.contributions {
        apply_one(
            section_prefix,
            tool,
            tool_table,
            merged,
            assertion,
            findings,
        );
    }
    if let Some(exact) = is_exactly_only(merged) {
        apply_exact_extras(section_prefix, tool, tool_table, merged, &exact, findings);
    }
}

/// Apply a single assertion variant.
fn apply_one(
    section_prefix: &str,
    tool: &str,
    tool_table: &mut Table,
    merged: &MergedAssertion<LintLevelsAssertion>,
    assertion: &LintLevelsAssertion,
    findings: &mut Vec<Finding>,
) {
    match assertion {
        LintLevelsAssertion::Contains(map) | LintLevelsAssertion::IsExactly(map) => {
            apply_contains(section_prefix, tool, tool_table, merged, map, findings);
        }
        LintLevelsAssertion::Excludes(map) => {
            apply_excludes(section_prefix, tool, tool_table, merged, map, findings);
        }
    }
}

/// `Contains` / `IsExactly` (per-key) — write each key in the form the
/// policy's `priority` slot implies (bare string when `None`, inline table
/// when `Some`).
#[expect(
    clippy::type_complexity,
    reason = "BTreeMap<String, (level, priority, message)> mirrors the assertion's value shape."
)]
fn apply_contains(
    section_prefix: &str,
    tool: &str,
    tool_table: &mut Table,
    merged: &MergedAssertion<LintLevelsAssertion>,
    map: &BTreeMap<String, (String, Option<i64>, String)>,
    findings: &mut Vec<Finding>,
) {
    for (lint, (level, priority, message)) in map {
        let on_disk = read_entry(tool_table, lint);
        if matches_policy(on_disk.as_ref(), level, *priority) {
            continue;
        }
        findings.push(Finding::Mismatch {
            path: format!("[{section_prefix}.{tool}].{lint}"),
            current: on_disk.map(|e| display_entry(&e)),
            expected: display_expected(level, *priority),
            message: message.clone(),
            severity: Severity::Error,
            attribution: contributors_for_lint(merged, lint),
        });
        tool_table[lint] = write_entry(level, *priority);
    }
}

/// `Excludes` — remove any of the named keys from the table.
fn apply_excludes(
    section_prefix: &str,
    tool: &str,
    tool_table: &mut Table,
    merged: &MergedAssertion<LintLevelsAssertion>,
    map: &BTreeMap<String, String>,
    findings: &mut Vec<Finding>,
) {
    for (lint, message) in map {
        if !tool_table.contains_key(lint) {
            continue;
        }
        let current = read_entry(tool_table, lint).map(|e| display_entry(&e));
        findings.push(Finding::Mismatch {
            path: format!("[{section_prefix}.{tool}].{lint}"),
            current,
            expected: "absent".to_owned(),
            message: message.clone(),
            severity: Severity::Error,
            attribution: contributors_for_lint(merged, lint),
        });
        let _ = tool_table.remove(lint);
    }
}

/// Drop on-disk keys not in any `IsExactly` contribution.
#[expect(
    clippy::type_complexity,
    reason = "BTreeMap<String, (level, priority, message)> mirrors the assertion's value shape."
)]
fn apply_exact_extras(
    section_prefix: &str,
    tool: &str,
    tool_table: &mut Table,
    merged: &MergedAssertion<LintLevelsAssertion>,
    exact: &BTreeMap<String, (String, Option<i64>, String)>,
    findings: &mut Vec<Finding>,
) {
    let on_disk: BTreeSet<String> = tool_table.iter().map(|(k, _)| k.to_owned()).collect();
    let allowed: BTreeSet<String> = exact.keys().cloned().collect();
    for extra in on_disk.difference(&allowed) {
        let current = read_entry(tool_table, extra).map(|e| display_entry(&e));
        findings.push(Finding::Mismatch {
            path: format!("[{section_prefix}.{tool}].{extra}"),
            current,
            expected: "absent (IsExactly)".to_owned(),
            message: NO_MESSAGE.to_owned(),
            severity: Severity::Error,
            attribution: contributors_for_assertion(merged),
        });
        let _ = tool_table.remove(extra);
    }
}

/// One on-disk lint-table entry, typed.
struct LintEntry {
    /// Lint level (`"deny"`, `"warn"`, `"allow"`, `"forbid"`).
    level: String,
    /// Priority field when the entry is inline-table form; `None` for
    /// bare-string form.
    priority: Option<i64>,
}

/// Parse the on-disk entry for `key`, accepting either bare string or
/// inline-table form.
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

/// Build the `toml_edit::Item` to write for one entry, picking bare string
/// or inline-table based on whether the policy supplied a priority.
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

/// Human-readable rendering of an on-disk entry for the finding's `current`
/// field.
fn display_entry(e: &LintEntry) -> String {
    e.priority.map_or_else(
        || e.level.clone(),
        |p| format!("{{ level = \"{level}\", priority = {p} }}", level = e.level),
    )
}

/// Human-readable rendering of the expected entry for the finding's
/// `expected` field.
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

/// Provenances of all contributions to this target.
fn contributors_for_assertion(merged: &MergedAssertion<LintLevelsAssertion>) -> Vec<Provenance> {
    all_provenances(merged)
}

/// Union of allowed keys if every contribution is `IsExactly`; otherwise `None`.
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
