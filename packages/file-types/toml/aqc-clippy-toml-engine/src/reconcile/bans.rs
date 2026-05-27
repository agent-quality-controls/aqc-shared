//! Reconciliation for clippy.toml ban tables. One implementation,
//! reused for `disallowed-methods`, `disallowed-types`, `disallowed-macros`.

use std::collections::BTreeSet;

use aqc_file_engine_core::{Finding, MergedAssertion, Provenance, Severity};
use toml_edit::{Array, DocumentMut, InlineTable, Item, Value};

use crate::reconcile::util::all_provenances;
use crate::requirement::{BanEntry, BansAssertion};

/// Apply every contribution against the named ban table.
#[expect(
    clippy::expect_used,
    reason = "or_insert(Item::Value(Value::Array(_))) guarantees as_array_mut returns Some"
)]
pub(crate) fn apply(
    doc: &mut DocumentMut,
    table_key: &str,
    merged: &MergedAssertion<BansAssertion>,
    findings: &mut Vec<Finding>,
) {
    let attribution = all_provenances(merged);
    let array = doc
        .entry(table_key)
        .or_insert(Item::Value(Value::Array(Array::new())))
        .as_array_mut()
        .expect("ban table is an array");
    let mut current_paths = collect_current_paths(array);
    for (_, assertion) in &merged.contributions {
        apply_one(
            table_key,
            array,
            &mut current_paths,
            assertion,
            &attribution,
            findings,
        );
    }
    if let Some(exact) = is_exactly_only(merged) {
        prune_extras(table_key, array, &exact, &attribution, findings);
    }
}

/// Dispatch one assertion variant.
fn apply_one(
    table_key: &str,
    array: &mut Array,
    current_paths: &mut BTreeSet<String>,
    assertion: &BansAssertion,
    attribution: &[Provenance],
    findings: &mut Vec<Finding>,
) {
    match assertion {
        BansAssertion::Contains(wanted) | BansAssertion::IsExactly(wanted) => {
            apply_contains(
                table_key,
                array,
                current_paths,
                wanted,
                attribution,
                findings,
            );
        }
        BansAssertion::Excludes(paths) => {
            apply_excludes(table_key, array, paths, attribution, findings);
        }
    }
}

/// Add any wanted entries that are not already present.
fn apply_contains(
    table_key: &str,
    array: &mut Array,
    current_paths: &mut BTreeSet<String>,
    wanted: &[BanEntry],
    attribution: &[Provenance],
    findings: &mut Vec<Finding>,
) {
    for entry in wanted {
        if current_paths.contains(&entry.path) {
            continue;
        }
        array.push(ban_value(entry));
        let _ = current_paths.insert(entry.path.clone());
        findings.push(Finding::Mismatch {
            path: format!("{table_key}[?path == \"{}\"]", entry.path),
            current: None,
            expected: format_entry(entry),
            severity: Severity::Error,
            attribution: attribution.to_vec(),
        });
    }
}

/// Remove any entries whose path matches any of `paths`.
fn apply_excludes(
    table_key: &str,
    array: &mut Array,
    paths: &BTreeSet<String>,
    attribution: &[Provenance],
    findings: &mut Vec<Finding>,
) {
    for path_to_remove in paths {
        let positions = positions_with_path(array, path_to_remove);
        for i in positions.into_iter().rev() {
            let _ = array.remove(i);
            findings.push(Finding::Mismatch {
                path: format!("{table_key}[?path == \"{path_to_remove}\"]"),
                current: Some(path_to_remove.clone()),
                expected: "absent".into(),
                severity: Severity::Error,
                attribution: attribution.to_vec(),
            });
        }
    }
}

/// Drop on-disk entries not in `exact`.
fn prune_extras(
    table_key: &str,
    array: &mut Array,
    exact: &[BanEntry],
    attribution: &[Provenance],
    findings: &mut Vec<Finding>,
) {
    let allowed: BTreeSet<String> = exact.iter().map(|e| e.path.clone()).collect();
    let mut indices_to_remove = Vec::new();
    for (i, v) in array.iter().enumerate() {
        if let Some(p) = read_entry_path(v) {
            if !allowed.contains(&p) {
                indices_to_remove.push((i, p));
            }
        }
    }
    for (i, p) in indices_to_remove.into_iter().rev() {
        let _ = array.remove(i);
        findings.push(Finding::Mismatch {
            path: format!("{table_key}[?path == \"{p}\"]"),
            current: Some(p),
            expected: "absent (IsExactly)".into(),
            severity: Severity::Error,
            attribution: attribution.to_vec(),
        });
    }
}

/// Scan the existing array and collect the set of paths already banned.
fn collect_current_paths(array: &Array) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for entry in array {
        if let Some(path) = read_entry_path(entry) {
            let _ = out.insert(path);
        }
    }
    out
}

/// Find every index whose entry's path matches `target`.
fn positions_with_path(array: &Array, target: &str) -> Vec<usize> {
    let mut out = Vec::new();
    for (i, v) in array.iter().enumerate() {
        if read_entry_path(v).as_deref() == Some(target) {
            out.push(i);
        }
    }
    out
}

/// Extract `path` from either a bare-string entry or an inline-table entry.
fn read_entry_path(item: &Value) -> Option<String> {
    if let Some(s) = item.as_str() {
        return Some(s.to_owned());
    }
    if let Some(t) = item.as_inline_table() {
        return t.get("path").and_then(Value::as_str).map(ToOwned::to_owned);
    }
    None
}

/// Build a TOML value for one ban entry. Uses bare string when no reason
/// is supplied; inline-table form when there is one.
fn ban_value(entry: &BanEntry) -> Value {
    entry.reason.as_ref().map_or_else(
        || Value::from(entry.path.as_str()),
        |reason| {
            let mut t = InlineTable::new();
            let _ = t.insert("path", Value::from(entry.path.as_str()));
            let _ = t.insert("reason", Value::from(reason.as_str()));
            Value::InlineTable(t)
        },
    )
}

/// Human-readable rendering of a `BanEntry` for finding messages.
fn format_entry(entry: &BanEntry) -> String {
    entry.reason.as_ref().map_or_else(
        || format!("path={}", entry.path),
        |r| format!("path={} reason={r}", entry.path),
    )
}

/// If every contribution is `IsExactly`, return the union of entries.
fn is_exactly_only(merged: &MergedAssertion<BansAssertion>) -> Option<Vec<BanEntry>> {
    let mut combined = Vec::new();
    for (_, a) in &merged.contributions {
        match a {
            BansAssertion::IsExactly(v) => combined.extend(v.iter().cloned()),
            BansAssertion::Contains(_) | BansAssertion::Excludes(_) => return None,
        }
    }
    if combined.is_empty() {
        None
    } else {
        Some(combined)
    }
}
