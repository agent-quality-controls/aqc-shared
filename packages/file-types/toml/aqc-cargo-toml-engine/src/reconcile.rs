//! Reconciliation logic for `CargoTomlEngine`. Walks merged assertions,
//! applies them to a `DocumentMut`, and emits `Finding`s for disagreements.

use std::collections::{BTreeMap, BTreeSet};

use aqc_file_engine_core::{Finding, MergedAssertion, Provenance, Severity};
use toml_edit::{DocumentMut, Item, Table, value};

use crate::requirement::{CargoTomlRequirement, LintLevelsAssertion};

/// Walk every target on `requirement`, applying its assertions to `doc`.
pub(crate) fn apply_requirement(
    doc: &mut DocumentMut,
    requirement: &CargoTomlRequirement,
    findings: &mut Vec<Finding>,
) {
    for (tool, merged) in &requirement.lints {
        apply_lints_table(doc, tool, merged, findings);
    }
}

/// Apply merged contributions for `[lints.<tool>]` to the document.
#[expect(
    clippy::expect_used,
    reason = "both expects follow an or_insert(Item::Table(_)) and are infallible by construction"
)]
fn apply_lints_table(
    doc: &mut DocumentMut,
    tool: &str,
    merged: &MergedAssertion<LintLevelsAssertion>,
    findings: &mut Vec<Finding>,
) {
    let lints_root = doc
        .entry("lints")
        .or_insert(Item::Table(Table::new()))
        .as_table_mut()
        .expect("lints is a table");
    let tool_table = lints_root
        .entry(tool)
        .or_insert(Item::Table(Table::new()))
        .as_table_mut()
        .expect("lints.<tool> is a table");

    for (_provenance, assertion) in &merged.contributions {
        apply_one_assertion(tool, tool_table, merged, assertion, findings);
    }

    if let Some(exact) = is_exactly_only(merged) {
        apply_is_exactly_extras(tool, tool_table, merged, &exact, findings);
    }
}

/// Apply a single contribution's assertion to `tool_table`.
fn apply_one_assertion(
    tool: &str,
    tool_table: &mut Table,
    merged: &MergedAssertion<LintLevelsAssertion>,
    assertion: &LintLevelsAssertion,
    findings: &mut Vec<Finding>,
) {
    match assertion {
        LintLevelsAssertion::Contains(map) | LintLevelsAssertion::IsExactly(map) => {
            apply_contains(tool, tool_table, merged, map, findings);
        }
        LintLevelsAssertion::Excludes(names) => {
            apply_excludes(tool, tool_table, merged, names, findings);
        }
    }
}

/// Apply a `Contains` / `IsExactly` mapping (per-key).
fn apply_contains(
    tool: &str,
    tool_table: &mut Table,
    merged: &MergedAssertion<LintLevelsAssertion>,
    map: &BTreeMap<String, String>,
    findings: &mut Vec<Finding>,
) {
    for (lint, level) in map {
        let current_level = current_str(tool_table, lint);
        if current_level.as_deref() == Some(level.as_str()) {
            continue;
        }
        findings.push(Finding::Mismatch {
            path: format!("[lints.{tool}].{lint}"),
            current: current_level,
            expected: level.clone(),
            severity: Severity::Error,
            attribution: contributors_for_lint(merged, lint),
        });
        tool_table[lint] = value(level.clone());
    }
}

/// Apply an `Excludes` set (each name must not be set).
fn apply_excludes(
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
            path: format!("[lints.{tool}].{lint}"),
            current: current_str(tool_table, lint),
            expected: "absent".to_owned(),
            severity: Severity::Error,
            attribution: contributors_for_lint(merged, lint),
        });
        let _ = tool_table.remove(lint);
    }
}

/// Remove on-disk entries that aren't covered by any `IsExactly` contribution.
fn apply_is_exactly_extras(
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
            path: format!("[lints.{tool}].{extra}"),
            current: current_str(tool_table, extra),
            expected: "absent (IsExactly)".to_owned(),
            severity: Severity::Error,
            attribution: contributors_for_assertion(merged),
        });
        let _ = tool_table.remove(extra);
    }
}

/// Read a lint key's value as a string, if it's currently set.
fn current_str(table: &Table, key: &str) -> Option<String> {
    table.get(key).and_then(Item::as_str).map(ToOwned::to_owned)
}

/// Collect provenances of every contribution that mentions `lint`.
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

/// Collect provenances of all contributions to this target.
fn contributors_for_assertion(merged: &MergedAssertion<LintLevelsAssertion>) -> Vec<Provenance> {
    merged
        .contributions
        .iter()
        .map(|(p, _)| p.clone())
        .collect()
}

/// If every contribution is `IsExactly`, return the union of allowed keys.
#[expect(
    clippy::type_complexity,
    reason = "BTreeMap<String, String> is the natural shape for the returned mapping; aliasing it hides what it is."
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
