//! Reconciliation for clippy.toml ban tables. One implementation,
//! reused for `disallowed-methods`, `disallowed-types`, `disallowed-macros`.

use std::collections::{BTreeMap, BTreeSet};

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
        BansAssertion::Excludes(map) => {
            apply_excludes(table_key, array, map, attribution, findings);
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
            message: entry.message.clone(),
            severity: Severity::Error,
            attribution: attribution.to_vec(),
        });
    }
}

/// Remove any entries whose path matches any of `paths`. Map value is the message.
fn apply_excludes(
    table_key: &str,
    array: &mut Array,
    map: &BTreeMap<String, String>,
    attribution: &[Provenance],
    findings: &mut Vec<Finding>,
) {
    for (path_to_remove, message) in map {
        let positions = positions_with_path(array, path_to_remove);
        for i in positions.into_iter().rev() {
            let _ = array.remove(i);
            findings.push(Finding::Mismatch {
                path: format!("{table_key}[?path == \"{path_to_remove}\"]"),
                current: Some(path_to_remove.clone()),
                expected: "absent".into(),
                message: message.clone(),
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
            message: String::new(),
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

/// Build a TOML value for one ban entry. Uses bare string when the
/// message is empty; inline-table form (path + reason) otherwise.
///
/// clippy's `disallowed-methods` schema names the field `reason`; we
/// keep that wire name even though our internal API calls it `message`.
fn ban_value(entry: &BanEntry) -> Value {
    if entry.message.is_empty() {
        Value::from(entry.path.as_str())
    } else {
        let mut t = InlineTable::new();
        let _ = t.insert("path", Value::from(entry.path.as_str()));
        let _ = t.insert("reason", Value::from(entry.message.as_str()));
        Value::InlineTable(t)
    }
}

/// Human-readable rendering of a `BanEntry` for finding `expected` text.
fn format_entry(entry: &BanEntry) -> String {
    if entry.message.is_empty() {
        format!("path={}", entry.path)
    } else {
        format!("path={} reason={}", entry.path, entry.message)
    }
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
