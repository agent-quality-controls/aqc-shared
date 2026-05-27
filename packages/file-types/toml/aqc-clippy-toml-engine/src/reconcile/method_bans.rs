//! Reconciliation for clippy.toml's `disallowed-methods` array.

use std::collections::BTreeSet;

use aqc_file_engine_core::{Finding, MergedAssertion, Provenance, Severity};
use toml_edit::{Array, DocumentMut, InlineTable, Item, Value};

use crate::reconcile::util::all_provenances;
use crate::requirement::{MethodBanEntry, MethodBansAssertion};

/// Apply every method-ban contribution.
#[expect(
    clippy::expect_used,
    reason = "or_insert(Item::Value(Value::Array(_))) guarantees as_array_mut returns Some"
)]
pub(crate) fn apply_method_bans(
    doc: &mut DocumentMut,
    merged: &MergedAssertion<MethodBansAssertion>,
    findings: &mut Vec<Finding>,
) {
    let attribution = all_provenances(merged);
    let array = doc
        .entry("disallowed-methods")
        .or_insert(Item::Value(Value::Array(Array::new())))
        .as_array_mut()
        .expect("disallowed-methods is an array");

    let mut current_paths = collect_current_paths(array);

    for (_, assertion) in &merged.contributions {
        apply_one(array, &mut current_paths, assertion, &attribution, findings);
    }

    if let Some(exact) = is_exactly_only(merged) {
        prune_extras(array, &exact, &attribution, findings);
    }
}

/// Dispatch one assertion variant.
fn apply_one(
    array: &mut Array,
    current_paths: &mut BTreeSet<String>,
    assertion: &MethodBansAssertion,
    attribution: &[Provenance],
    findings: &mut Vec<Finding>,
) {
    match assertion {
        MethodBansAssertion::Contains(wanted) | MethodBansAssertion::IsExactly(wanted) => {
            apply_contains(array, current_paths, wanted, attribution, findings);
        }
        MethodBansAssertion::Excludes(paths) => {
            apply_excludes(array, paths, attribution, findings);
        }
    }
}

/// Add any wanted entries that are not already present.
fn apply_contains(
    array: &mut Array,
    current_paths: &mut BTreeSet<String>,
    wanted: &[MethodBanEntry],
    attribution: &[Provenance],
    findings: &mut Vec<Finding>,
) {
    for entry in wanted {
        if current_paths.contains(&entry.path) {
            continue;
        }
        array.push(method_ban_value(entry));
        let _ = current_paths.insert(entry.path.clone());
        findings.push(Finding::Mismatch {
            path: format!("disallowed-methods[?path == \"{}\"]", entry.path),
            current: None,
            expected: format!("path={} reason={}", entry.path, entry.reason),
            severity: Severity::Error,
            attribution: attribution.to_vec(),
        });
    }
}

/// Remove any entries whose path matches any of `paths`.
fn apply_excludes(
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
                path: format!("disallowed-methods[?path == \"{path_to_remove}\"]"),
                current: Some(path_to_remove.clone()),
                expected: "absent".into(),
                severity: Severity::Error,
                attribution: attribution.to_vec(),
            });
        }
    }
}

/// Drop on-disk entries that aren't in `exact`.
fn prune_extras(
    array: &mut Array,
    exact: &[MethodBanEntry],
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
            path: format!("disallowed-methods[?path == \"{p}\"]"),
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

/// Build the inline-table form of a method ban.
fn method_ban_value(entry: &MethodBanEntry) -> Value {
    let mut t = InlineTable::new();
    let _ = t.insert("path", Value::from(entry.path.as_str()));
    let _ = t.insert("reason", Value::from(entry.reason.as_str()));
    Value::InlineTable(t)
}

/// If every contribution is `IsExactly`, return the union of entries.
fn is_exactly_only(merged: &MergedAssertion<MethodBansAssertion>) -> Option<Vec<MethodBanEntry>> {
    let mut combined = Vec::new();
    for (_, a) in &merged.contributions {
        match a {
            MethodBansAssertion::IsExactly(v) => combined.extend(v.iter().cloned()),
            MethodBansAssertion::Contains(_) | MethodBansAssertion::Excludes(_) => return None,
        }
    }
    if combined.is_empty() {
        None
    } else {
        Some(combined)
    }
}
