//! Reconcile dependency tables: `[dependencies]` / `[dev-dependencies]` /
//! `[build-dependencies]`, their `[target.'cfg'.*]` variants,
//! `[workspace.dependencies]`, and `[patch.<registry>]` (same vocabulary).
//!
//! The core `apply_set` is shared by every dependency-shaped target; callers
//! pass the table's path segments and any extra generability rules (workspace
//! deps forbid `optional`).
//!
//! Lazy: an `Excludes`-only requirement against a missing table writes nothing.
//! A `Contains` entry with no source (cargo would reject it) is check-only.

#![expect(
    clippy::type_complexity,
    reason = "Collected assertions are plainly Vec<(Provenance, A)> and per-key maps of them; the shapes are declared openly at every signature instead of hidden behind wrapper types or aliases (taxonomy decision 2026-06-07)."
)]
use std::collections::{BTreeMap, BTreeSet};

use aqc_file_engine_core::{Finding, Provenance, Severity};
use toml_edit::{Array, DocumentMut, InlineTable, Item, Table, Value};

use crate::reconcile::util::{all_provenances, ensure_table_at, table_at, table_at_mut};
use crate::requirement::{DependencyScope, DependencySetAssertion, DependencySpec};

/// Extra generability rule for a dependency-shaped table.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum SetRule {
    /// Standard dependency table.
    Standard,
    /// `[workspace.dependencies]`: `optional` is invalid (cargo rule).
    WorkspaceDeps,
}

/// Apply every scoped dependency-table contribution.
pub(crate) fn apply(
    doc: &mut DocumentMut,
    merged_by_scope: &BTreeMap<DependencyScope, Vec<(Provenance, DependencySetAssertion)>>,
    findings: &mut Vec<Finding>,
) {
    for (scope, merged) in merged_by_scope {
        let path = scope_path(scope);
        apply_set(
            doc,
            &path,
            &scope.table_path(),
            SetRule::Standard,
            merged,
            findings,
        );
    }
}

/// The path segments of a `DependencyScope`'s table.
fn scope_path(scope: &DependencyScope) -> Vec<String> {
    let kind = match scope.kind {
        crate::requirement::DependencyKind::Normal => "dependencies",
        crate::requirement::DependencyKind::Dev => "dev-dependencies",
        crate::requirement::DependencyKind::Build => "build-dependencies",
    };
    scope.target.as_ref().map_or_else(
        || vec![kind.to_owned()],
        |t| vec!["target".to_owned(), t.clone(), kind.to_owned()],
    )
}

/// Apply one `DependencySetAssertion` to the table at `path`.
///
/// `display_path` is the finding-path prefix (e.g. `[dependencies]`). `rule`
/// carries any extra generability constraint.
pub(crate) fn apply_set(
    doc: &mut DocumentMut,
    path: &[String],
    display_path: &str,
    rule: SetRule,
    merged: &Vec<(Provenance, DependencySetAssertion)>,
    findings: &mut Vec<Finding>,
) {
    let attribution = all_provenances(merged);
    for (_, assertion) in merged {
        match assertion {
            DependencySetAssertion::Contains(map) | DependencySetAssertion::IsExactly(map) => {
                apply_contains(doc, path, display_path, rule, map, &attribution, findings);
            }
            DependencySetAssertion::Excludes(map) => {
                apply_excludes(doc, path, display_path, map, &attribution, findings);
            }
        }
    }
    if let Some(allowed) = is_exactly_only(merged) {
        apply_exact_extras(doc, path, display_path, &allowed, &attribution, findings);
    }
}

/// Each `(name, spec)` must be present and partial-match.
fn apply_contains(
    doc: &mut DocumentMut,
    path: &[String],
    display_path: &str,
    rule: SetRule,
    map: &BTreeMap<String, (DependencySpec, String)>,
    attribution: &[Provenance],
    findings: &mut Vec<Finding>,
) {
    for (name, (spec, msg)) in map {
        if rule == SetRule::WorkspaceDeps && spec.optional.is_some() {
            findings.push(Finding::InvalidRequirements {
                key: format!("{display_path}.{name}"),
                message: format!("optional is invalid in [workspace.dependencies].{name}. {msg}"),
                contributors: attribution
                    .iter()
                    .map(|p| (p.policy.clone(), format!("{name} (optional)")))
                    .collect(),
            });
            continue;
        }
        let current = table_at(doc, path).and_then(|t| read_spec(t, name));
        if current.as_ref().is_some_and(|c| spec_matches(spec, c)) {
            continue;
        }
        let writable = spec.has_source();
        findings.push(Finding::Mismatch {
            key: format!("{display_path}.{name}"),
            current: current.as_ref().map(|s| format!("{s:?}")),
            expected: if writable {
                format!("{spec:?}")
            } else {
                format!("{spec:?} (no source: check-only)")
            },
            message: msg.clone(),
            severity: Severity::Error,
            attribution: attribution.to_vec(),
        });
        if writable {
            ensure_table_at(doc, path)[name] = spec_to_item(spec);
        }
    }
}

/// Each named entry must be absent (vacuous when the table is missing).
fn apply_excludes(
    doc: &mut DocumentMut,
    path: &[String],
    display_path: &str,
    map: &BTreeMap<String, String>,
    attribution: &[Provenance],
    findings: &mut Vec<Finding>,
) {
    for (name, msg) in map {
        let Some(current) = table_at(doc, path).and_then(|t| read_spec(t, name)) else {
            continue;
        };
        findings.push(Finding::Mismatch {
            key: format!("{display_path}.{name}"),
            current: Some(format!("{current:?}")),
            expected: "absent".to_owned(),
            message: msg.clone(),
            severity: Severity::Error,
            attribution: attribution.to_vec(),
        });
        if let Some(t) = table_at_mut(doc, path) {
            let _ = t.remove(name);
        }
    }
}

/// Drop on-disk entries not in the `IsExactly` union.
fn apply_exact_extras(
    doc: &mut DocumentMut,
    path: &[String],
    display_path: &str,
    allowed: &BTreeSet<String>,
    attribution: &[Provenance],
    findings: &mut Vec<Finding>,
) {
    let Some(table) = table_at(doc, path) else {
        return;
    };
    let on_disk: BTreeSet<String> = table.iter().map(|(k, _)| k.to_owned()).collect();
    let extras: Vec<String> = on_disk.difference(allowed).cloned().collect();
    for extra in &extras {
        let current = table_at(doc, path).and_then(|t| read_spec(t, extra));
        findings.push(Finding::Mismatch {
            key: format!("{display_path}.{extra}"),
            current: current.map(|s| format!("{s:?}")),
            expected: "absent (IsExactly)".to_owned(),
            message: String::new(),
            severity: Severity::Error,
            attribution: attribution.to_vec(),
        });
        if let Some(t) = table_at_mut(doc, path) {
            let _ = t.remove(extra);
        }
    }
}

/// True when every field the spec sets equals the on-disk entry (partial match).
fn spec_matches(spec: &DependencySpec, current: &DependencySpec) -> bool {
    spec.version
        .as_ref()
        .is_none_or(|v| Some(v) == current.version.as_ref())
        && (spec.features.is_empty() || spec.features == current.features)
        && spec
            .default_features
            .is_none_or(|b| Some(b) == current.default_features)
        && spec.optional.is_none_or(|b| Some(b) == current.optional)
        && spec.workspace.is_none_or(|b| Some(b) == current.workspace)
        && spec
            .path
            .as_ref()
            .is_none_or(|v| Some(v) == current.path.as_ref())
        && spec
            .git
            .as_ref()
            .is_none_or(|v| Some(v) == current.git.as_ref())
        && spec
            .branch
            .as_ref()
            .is_none_or(|v| Some(v) == current.branch.as_ref())
        && spec
            .tag
            .as_ref()
            .is_none_or(|v| Some(v) == current.tag.as_ref())
        && spec
            .rev
            .as_ref()
            .is_none_or(|v| Some(v) == current.rev.as_ref())
        && spec
            .registry
            .as_ref()
            .is_none_or(|v| Some(v) == current.registry.as_ref())
        && spec
            .package
            .as_ref()
            .is_none_or(|v| Some(v) == current.package.as_ref())
}

/// Read an existing dependency entry into a `DependencySpec`. Handles the
/// bare-string form (`serde = "1"`) and the inline-table form.
fn read_spec(table: &Table, name: &str) -> Option<DependencySpec> {
    let item = table.get(name)?;
    if let Some(s) = item.as_str() {
        return Some(DependencySpec {
            version: Some(s.to_owned()),
            ..DependencySpec::default()
        });
    }
    let inline = item.as_inline_table()?;
    let str_field = |k: &str| inline.get(k).and_then(Value::as_str).map(ToOwned::to_owned);
    Some(DependencySpec {
        version: str_field("version"),
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
        workspace: inline.get("workspace").and_then(Value::as_bool),
        path: str_field("path"),
        git: str_field("git"),
        branch: str_field("branch"),
        tag: str_field("tag"),
        rev: str_field("rev"),
        registry: str_field("registry"),
        package: str_field("package"),
    })
}

/// Render a `DependencySpec` to write: bare string when only `version` is set,
/// inline table otherwise (including the `workspace = true` form).
fn spec_to_item(spec: &DependencySpec) -> Item {
    if is_version_only(spec) {
        if let Some(v) = &spec.version {
            return Item::Value(Value::from(v.as_str()));
        }
    }
    Item::Value(Value::InlineTable(spec_to_inline(spec)))
}

/// True when `version` is the only set field (string-shorthand candidate).
fn is_version_only(spec: &DependencySpec) -> bool {
    spec.version.is_some()
        && spec.features.is_empty()
        && spec.default_features.is_none()
        && spec.optional.is_none()
        && spec.workspace.is_none()
        && spec.path.is_none()
        && spec.git.is_none()
        && spec.branch.is_none()
        && spec.tag.is_none()
        && spec.rev.is_none()
        && spec.registry.is_none()
        && spec.package.is_none()
}

/// Render a `DependencySpec` as an inline TOML table.
fn spec_to_inline(spec: &DependencySpec) -> InlineTable {
    let mut t = InlineTable::new();
    put_str(&mut t, "version", spec.version.as_ref());
    if !spec.features.is_empty() {
        let mut arr = Array::new();
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
    if let Some(b) = spec.workspace {
        let _ = t.insert("workspace", Value::from(b));
    }
    put_str(&mut t, "path", spec.path.as_ref());
    put_str(&mut t, "git", spec.git.as_ref());
    put_str(&mut t, "branch", spec.branch.as_ref());
    put_str(&mut t, "tag", spec.tag.as_ref());
    put_str(&mut t, "rev", spec.rev.as_ref());
    put_str(&mut t, "registry", spec.registry.as_ref());
    put_str(&mut t, "package", spec.package.as_ref());
    t
}

/// Insert `k = "<v>"` into the inline table when the field is set.
fn put_str(t: &mut InlineTable, k: &str, v: Option<&String>) {
    if let Some(s) = v {
        let _ = t.insert(k, Value::from(s.as_str()));
    }
}

/// Union of allowed names if every contribution is `IsExactly`; else `None`.
fn is_exactly_only(merged: &Vec<(Provenance, DependencySetAssertion)>) -> Option<BTreeSet<String>> {
    let mut combined: BTreeSet<String> = BTreeSet::new();
    for (_, assertion) in merged {
        match assertion {
            DependencySetAssertion::IsExactly(map) => combined.extend(map.keys().cloned()),
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
