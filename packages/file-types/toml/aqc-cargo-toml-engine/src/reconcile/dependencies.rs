//! Reconcile `[dependencies]`, `[dev-dependencies]`, `[build-dependencies]`.

use std::collections::{BTreeMap, BTreeSet};

use aqc_file_engine_core::{Finding, MergedAssertion, Provenance, Severity};
use toml_edit::{InlineTable, Item, Table, Value};

use crate::reconcile::util::{all_provenances, get_or_create_table_mut};
use crate::requirement::{DepKind, DependencySetAssertion, DependencySpec};

/// Apply every dependencies contribution.
#[expect(
    clippy::type_complexity,
    reason = "BTreeMap<String, MergedAssertion<...>> is the natural section input shape"
)]
pub(crate) fn apply(
    doc: &mut toml_edit::DocumentMut,
    merged_by_kind: &BTreeMap<DepKind, MergedAssertion<DependencySetAssertion>>,
    findings: &mut Vec<Finding>,
) {
    for (kind, merged) in merged_by_kind {
        let table_key = section_key(*kind);
        let table = get_or_create_table_mut(doc, table_key);
        apply_kind(table_key, table, merged, findings);
    }
}

/// Apply contributions for one dependency kind's table.
fn apply_kind(
    table_key: &str,
    table: &mut Table,
    merged: &MergedAssertion<DependencySetAssertion>,
    findings: &mut Vec<Finding>,
) {
    let attribution = all_provenances(merged);
    for (_, assertion) in &merged.contributions {
        match assertion {
            DependencySetAssertion::Contains(map) | DependencySetAssertion::IsExactly(map) => {
                apply_contains(table_key, table, map, &attribution, findings);
            }
            DependencySetAssertion::Excludes(names) => {
                apply_excludes(table_key, table, names, &attribution, findings);
            }
        }
    }
    if let Some(allowed) = is_exactly_only(merged) {
        apply_exact_extras(table_key, table, &allowed, &attribution, findings);
    }
}

/// Each `(name, spec)` must be present and match.
fn apply_contains(
    table_key: &str,
    table: &mut Table,
    map: &BTreeMap<String, DependencySpec>,
    attribution: &[Provenance],
    findings: &mut Vec<Finding>,
) {
    for (name, spec) in map {
        let current = read_spec(table, name);
        if current.as_ref() == Some(spec) {
            continue;
        }
        findings.push(Finding::Mismatch {
            path: format!("[{table_key}].{name}"),
            current: current.as_ref().map(|s| format!("{s:?}")),
            expected: format!("{spec:?}"),
            message: String::new(),
            severity: Severity::Error,
            attribution: attribution.to_vec(),
        });
        table[name] = Item::Value(Value::InlineTable(spec_to_inline(spec)));
    }
}

/// Each named entry must be absent.
fn apply_excludes(
    table_key: &str,
    table: &mut Table,
    names: &BTreeSet<String>,
    attribution: &[Provenance],
    findings: &mut Vec<Finding>,
) {
    for name in names {
        if !table.contains_key(name) {
            continue;
        }
        findings.push(Finding::Mismatch {
            path: format!("[{table_key}].{name}"),
            current: Some(format!("{:?}", read_spec(table, name))),
            expected: "absent".into(),
            message: String::new(),
            severity: Severity::Error,
            attribution: attribution.to_vec(),
        });
        let _ = table.remove(name);
    }
}

/// Drop on-disk entries not in the `IsExactly` union.
fn apply_exact_extras(
    table_key: &str,
    table: &mut Table,
    allowed: &BTreeMap<String, DependencySpec>,
    attribution: &[Provenance],
    findings: &mut Vec<Finding>,
) {
    let on_disk: BTreeSet<String> = table.iter().map(|(k, _)| k.to_owned()).collect();
    let allowed_keys: BTreeSet<String> = allowed.keys().cloned().collect();
    for extra in on_disk.difference(&allowed_keys) {
        findings.push(Finding::Mismatch {
            path: format!("[{table_key}].{extra}"),
            current: Some(format!("{:?}", read_spec(table, extra))),
            expected: "absent (IsExactly)".into(),
            message: String::new(),
            severity: Severity::Error,
            attribution: attribution.to_vec(),
        });
        let _ = table.remove(extra);
    }
}

/// Map `DepKind` to its Cargo.toml table name.
const fn section_key(kind: DepKind) -> &'static str {
    match kind {
        DepKind::Normal => "dependencies",
        DepKind::Dev => "dev-dependencies",
        DepKind::Build => "build-dependencies",
    }
}

/// Read an existing dependency entry into a `DependencySpec`. Handles both
/// the bare-string form (`serde = "1"`) and the inline-table form.
fn read_spec(table: &Table, name: &str) -> Option<DependencySpec> {
    let item = table.get(name)?;
    if let Some(s) = item.as_str() {
        return Some(DependencySpec {
            version: Some(s.to_owned()),
            ..DependencySpec::default()
        });
    }
    let inline = item.as_inline_table()?;
    Some(DependencySpec {
        version: inline
            .get("version")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned),
        features: inline
            .get("features")
            .and_then(Value::as_array)
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(ToOwned::to_owned))
                    .collect()
            })
            .unwrap_or_default(),
        default_features: inline.get("default-features").and_then(Value::as_bool),
        optional: inline.get("optional").and_then(Value::as_bool),
    })
}

/// Render a `DependencySpec` as an inline TOML table.
fn spec_to_inline(spec: &DependencySpec) -> InlineTable {
    let mut t = InlineTable::new();
    if let Some(v) = &spec.version {
        let _ = t.insert("version", Value::from(v.as_str()));
    }
    if !spec.features.is_empty() {
        let mut arr = toml_edit::Array::new();
        for f in &spec.features {
            arr.push(Value::from(f.as_str()));
        }
        let _ = t.insert("features", Value::Array(arr));
    }
    if let Some(b) = spec.default_features {
        let _ = t.insert("default-features", Value::from(b));
    }
    if let Some(b) = spec.optional {
        let _ = t.insert("optional", Value::from(b));
    }
    t
}

/// If every contribution is `IsExactly`, return the union; else None.
#[expect(
    clippy::type_complexity,
    reason = "BTreeMap<String, DependencySpec> is the natural shape for the returned mapping."
)]
fn is_exactly_only(
    merged: &MergedAssertion<DependencySetAssertion>,
) -> Option<BTreeMap<String, DependencySpec>> {
    let mut combined: BTreeMap<String, DependencySpec> = BTreeMap::new();
    for (_, assertion) in &merged.contributions {
        match assertion {
            DependencySetAssertion::IsExactly(map) => {
                for (k, v) in map {
                    let _ = combined.insert(k.clone(), v.clone());
                }
            }
            DependencySetAssertion::Contains(_) | DependencySetAssertion::Excludes(_) => {
                return None;
            }
        }
    }
    if combined.is_empty() {
        None
    } else {
        Some(combined)
    }
}
