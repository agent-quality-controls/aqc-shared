//! Reconcile `[lints.<tool>]` tables.

use std::collections::{BTreeMap, BTreeSet};

use aqc_file_engine_core::{Finding, MergedAssertion, Provenance, Severity};
use toml_edit::{Item, Table, value};

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
        LintLevelsAssertion::Excludes(names) => {
            apply_excludes(section_prefix, tool, tool_table, merged, names, findings);
        }
    }
}

/// `Contains` / `IsExactly` (per-key) — set each key to its required level.
fn apply_contains(
    section_prefix: &str,
    tool: &str,
    tool_table: &mut Table,
    merged: &MergedAssertion<LintLevelsAssertion>,
    map: &BTreeMap<String, String>,
    findings: &mut Vec<Finding>,
) {
    for (lint, level) in map {
        let current = current_str(tool_table, lint);
        if current.as_deref() == Some(level.as_str()) {
            continue;
        }
        findings.push(Finding::Mismatch {
            path: format!("[{section_prefix}.{tool}].{lint}"),
            current,
            expected: level.clone(),
            severity: Severity::Error,
            attribution: contributors_for_lint(merged, lint),
        });
        tool_table[lint] = value(level.clone());
    }
}

/// `Excludes` — remove any of the named keys from the table.
fn apply_excludes(
    section_prefix: &str,
    tool: &str,
    tool_table: &mut Table,
    merged: &MergedAssertion<LintLevelsAssertion>,
    names: &BTreeSet<String>,
    findings: &mut Vec<Finding>,
) {
    for lint in names {
        if !tool_table.contains_key(lint) {
            continue;
        }
        findings.push(Finding::Mismatch {
            path: format!("[{section_prefix}.{tool}].{lint}"),
            current: current_str(tool_table, lint),
            expected: "absent".to_owned(),
            severity: Severity::Error,
            attribution: contributors_for_lint(merged, lint),
        });
        let _ = tool_table.remove(lint);
    }
}

/// Drop on-disk keys not in any `IsExactly` contribution.
fn apply_exact_extras(
    section_prefix: &str,
    tool: &str,
    tool_table: &mut Table,
    merged: &MergedAssertion<LintLevelsAssertion>,
    exact: &BTreeMap<String, String>,
    findings: &mut Vec<Finding>,
) {
    let on_disk: BTreeSet<String> = tool_table.iter().map(|(k, _)| k.to_owned()).collect();
    let allowed: BTreeSet<String> = exact.keys().cloned().collect();
    for extra in on_disk.difference(&allowed) {
        findings.push(Finding::Mismatch {
            path: format!("[{section_prefix}.{tool}].{extra}"),
            current: current_str(tool_table, extra),
            expected: "absent (IsExactly)".to_owned(),
            severity: Severity::Error,
            attribution: contributors_for_assertion(merged),
        });
        let _ = tool_table.remove(extra);
    }
}

/// Read the current string value of `key` in `table`, if any.
fn current_str(table: &Table, key: &str) -> Option<String> {
    table.get(key).and_then(Item::as_str).map(ToOwned::to_owned)
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
            LintLevelsAssertion::Excludes(names) => names.contains(lint),
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
    reason = "BTreeMap<String, String> is the natural shape for the returned mapping."
)]
fn is_exactly_only(
    merged: &MergedAssertion<LintLevelsAssertion>,
) -> Option<BTreeMap<String, String>> {
    let mut combined: BTreeMap<String, String> = BTreeMap::new();
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
