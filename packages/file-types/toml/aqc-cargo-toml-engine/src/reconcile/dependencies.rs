//! Reconcile dependency tables: `[dependencies]` / `[dev-dependencies]` /
//! `[build-dependencies]`, their `[target.'cfg'.*]` variants,
//! `[workspace.dependencies]`, and `[patch.<registry>]` (same vocabulary).
//!
//! The core `apply_set` is shared by every dependency-shaped target; callers
//! pass the table's path segments and any extra generability rules (workspace
//! deps forbid `optional`).
//!
//! Lazy: a ban-only requirement against a missing table writes nothing.
//! A `Contains` entry with no source (cargo would reject it) is check-only.

use std::collections::{BTreeMap, BTreeSet};

use aqc_file_engine_core::{Finding, Provenance, ResolvedItemRequirements, Severity};
use toml_edit::{Array, DocumentMut, InlineTable, Item, Table, Value};

use crate::reconcile::util::{ensure_table_at, table_at, table_at_mut};
use crate::requirement::{DependencyRequirement, DependencyScope, DependencySpec};

/// Extra generability rule for a dependency-shaped table.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum SetRule {
    /// Standard dependency table.
    Standard,
    /// `[workspace.dependencies]`: `optional` is invalid (cargo rule).
    WorkspaceDeps,
    /// `[patch.<registry>]`: package-only requirements are not writable.
    Patch,
}

/// Apply every scoped dependency-table requirement.
pub(crate) fn apply(
    doc: &mut DocumentMut,
    merged_by_scope: &BTreeMap<DependencyScope, ResolvedItemRequirements<DependencyRequirement>>,
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

/// Apply one dependency table to the table at `path`.
pub(crate) fn apply_set(
    doc: &mut DocumentMut,
    path: &[String],
    display_path: &str,
    rule: SetRule,
    merged: &ResolvedItemRequirements<DependencyRequirement>,
    findings: &mut Vec<Finding>,
) {
    let required_file_keys = required_file_keys(merged);
    for entry in merged.required.values() {
        let attribution = entry
            .collected
            .iter()
            .map(|(prov, _)| prov.clone())
            .collect::<Vec<_>>();
        let msg = entry
            .collected
            .first()
            .map(|(_, (_, msg))| msg.clone())
            .unwrap_or_default();
        apply_required(
            doc,
            path,
            display_path,
            rule,
            &entry.merged,
            &required_file_keys,
            &msg,
            &attribution,
            findings,
        );
    }
    for entry in merged.banned.values() {
        let attribution = entry
            .collected
            .iter()
            .map(|(prov, _)| prov.clone())
            .collect::<Vec<_>>();
        let msg = entry
            .collected
            .first()
            .map(|(_, msg)| msg.clone())
            .unwrap_or_default();
        apply_banned(
            doc,
            path,
            display_path,
            &entry.merged,
            &msg,
            &attribution,
            findings,
        );
    }
    if !merged.closed_by.is_empty() {
        let allowed = merged
            .required
            .values()
            .map(|entry| entry.merged.clone())
            .collect::<Vec<_>>();
        let attribution = merged
            .closed_by
            .iter()
            .map(|(prov, _)| prov.clone())
            .collect::<Vec<_>>();
        apply_exact_extras(doc, path, display_path, &allowed, &attribution, findings);
    }
}

/// Each dependency requirement must be present and partial-match.
fn apply_required(
    doc: &mut DocumentMut,
    path: &[String],
    display_path: &str,
    rule: SetRule,
    requirement: &DependencyRequirement,
    required_file_keys: &RequiredFileKeys,
    msg: &str,
    attribution: &[Provenance],
    findings: &mut Vec<Finding>,
) {
    let spec = &requirement.value;
    let name = requirement
        .file_key
        .as_deref()
        .or(requirement.value.package.as_deref())
        .unwrap_or("<unknown>");
    if rule == SetRule::WorkspaceDeps && spec.optional.is_some() {
        findings.push(Finding::InvalidRequirements {
            key: format!("{display_path}.{name}"),
            message: format!("optional is invalid in [workspace.dependencies].{name}. {msg}"),
            contributors: attribution
                .iter()
                .map(|p| (p.policy.clone(), format!("{name} optional")))
                .collect(),
        });
        return;
    }
    let current = table_at(doc, path).and_then(|t| {
        if let Some(file_key) = requirement.file_key.as_deref() {
            read_spec(t, file_key).map(|spec| (file_key.to_owned(), spec))
        } else {
            let matches = find_all_by_package(t, requirement.value.package.as_deref()?);
            if matches
                .iter()
                .any(|(_, current_spec)| spec_matches(spec, current_spec))
            {
                return Some((String::new(), spec.clone()));
            }
            matches.into_iter().next()
        }
    });
    if current
        .as_ref()
        .is_some_and(|(_, current_spec)| spec_matches(spec, current_spec))
    {
        return;
    }
    let write_key = requirement
        .file_key
        .clone()
        .or_else(|| requirement.value.package.clone());
    let writable = spec.has_source()
        && write_key.is_some()
        && (rule != SetRule::Patch || requirement.file_key.is_some());
    if spec.has_source() && rule == SetRule::Patch && requirement.file_key.is_none() {
        findings.push(Finding::UnwritableRequiredKey {
            key: format!("{display_path}.{name}"),
            expected: format!("{spec:?}"),
            attribution: attribution.to_vec(),
        });
        return;
    }
    if spec.has_source()
        && requirement
            .file_key
            .as_deref()
            .is_some_and(|key| required_file_keys.has_conflicting_packages(key))
    {
        findings.push(Finding::UnwritableRequiredKey {
            key: format!("{display_path}.{name}"),
            expected: format!("{spec:?}"),
            attribution: attribution.to_vec(),
        });
        return;
    }
    if spec.has_source()
        && requirement.file_key.is_none()
        && write_key.as_deref().is_some_and(|key| {
            package_write_key_is_reserved(doc, path, key, spec, required_file_keys)
        })
    {
        findings.push(Finding::UnwritableRequiredKey {
            key: format!("{display_path}.{name}"),
            expected: format!("{spec:?}"),
            attribution: attribution.to_vec(),
        });
        return;
    }
    findings.push(Finding::Mismatch {
        key: format!("{display_path}.{name}"),
        current: current.as_ref().map(|(_, s)| format!("{s:?}")),
        expected: if writable {
            format!("{spec:?}")
        } else {
            format!("{spec:?} (no source: check-only)")
        },
        message: msg.to_owned(),
        severity: Severity::Error,
        attribution: attribution.to_vec(),
    });
    if writable {
        if let Some(write_key) = write_key {
            let write_spec = spec_for_write_key(spec, &write_key);
            ensure_table_at(doc, path)[&write_key] = spec_to_item(&write_spec);
        }
    }
}

#[derive(Debug, Default)]
struct RequiredFileKeys {
    packages_by_key: BTreeMap<String, BTreeSet<String>>,
}

impl RequiredFileKeys {
    fn contains(&self, file_key: &str) -> bool {
        self.packages_by_key.contains_key(file_key)
    }

    fn has_conflicting_packages(&self, file_key: &str) -> bool {
        self.packages_by_key
            .get(file_key)
            .is_some_and(|packages| packages.len() > 1)
    }
}

fn required_file_keys(
    merged: &ResolvedItemRequirements<DependencyRequirement>,
) -> RequiredFileKeys {
    let mut out = RequiredFileKeys::default();
    for entry in merged.required.values() {
        let Some(file_key) = entry.merged.file_key.as_ref() else {
            continue;
        };
        let effective_package = entry
            .merged
            .value
            .package
            .as_deref()
            .unwrap_or(file_key)
            .to_owned();
        let _ = out
            .packages_by_key
            .entry(file_key.clone())
            .or_default()
            .insert(effective_package);
    }
    out
}

fn package_write_key_is_reserved(
    doc: &DocumentMut,
    path: &[String],
    write_key: &str,
    spec: &DependencySpec,
    required_file_keys: &RequiredFileKeys,
) -> bool {
    if required_file_keys.contains(write_key) {
        return true;
    }
    let Some(package) = spec.package.as_deref() else {
        return false;
    };
    table_at(doc, path)
        .and_then(|table| read_spec(table, write_key))
        .is_some_and(|current| effective_package(write_key, &current) != package)
}

/// Each named entry must be absent (vacuous when the table is missing).
fn apply_banned(
    doc: &mut DocumentMut,
    path: &[String],
    display_path: &str,
    requirement: &DependencyRequirement,
    msg: &str,
    attribution: &[Provenance],
    findings: &mut Vec<Finding>,
) {
    let matches = table_at(doc, path)
        .map(|table| read_banned_matches(table, requirement))
        .unwrap_or_default();
    for (name, current) in matches {
        findings.push(Finding::Mismatch {
            key: format!("{display_path}.{name}"),
            current: Some(format!("{current:?}")),
            expected: "absent".to_owned(),
            message: msg.to_owned(),
            severity: Severity::Error,
            attribution: attribution.to_vec(),
        });
        if let Some(t) = table_at_mut(doc, path) {
            let _ = t.remove(&name);
        }
    }
}

fn read_banned_matches(
    table: &Table,
    requirement: &DependencyRequirement,
) -> Vec<(String, DependencySpec)> {
    if let Some(file_key) = requirement.file_key.as_deref() {
        return read_spec(table, file_key)
            .map(|spec| vec![(file_key.to_owned(), spec)])
            .unwrap_or_default();
    }
    let Some(package) = requirement.value.package.as_deref() else {
        return Vec::new();
    };
    find_all_by_package(table, package)
}

/// Drop on-disk entries not allowed by the closed collection.
fn apply_exact_extras(
    doc: &mut DocumentMut,
    path: &[String],
    display_path: &str,
    allowed: &[DependencyRequirement],
    attribution: &[Provenance],
    findings: &mut Vec<Finding>,
) {
    let Some(table) = table_at(doc, path) else {
        return;
    };
    let extras = table
        .iter()
        .filter_map(|(file_key, _)| {
            let spec = read_spec(table, file_key)?;
            let effective_package = effective_package(file_key, &spec).to_owned();
            let allowed = allowed.iter().any(|requirement| {
                requirement_matches_file_item(requirement, file_key, &effective_package)
            });
            (!allowed).then(|| (file_key.to_owned(), spec))
        })
        .collect::<Vec<_>>();
    for (extra, current) in &extras {
        findings.push(Finding::Mismatch {
            key: format!("{display_path}.{extra}"),
            current: Some(format!("{current:?}")),
            expected: "absent (closed collection)".to_owned(),
            message: String::new(),
            severity: Severity::Error,
            attribution: attribution.to_vec(),
        });
        if let Some(t) = table_at_mut(doc, path) {
            let _ = t.remove(extra.as_str());
        }
    }
}

fn requirement_matches_file_item(
    requirement: &DependencyRequirement,
    file_key: &str,
    effective_package: &str,
) -> bool {
    requirement.file_key.as_deref().map_or_else(
        || requirement.value.package.as_deref() == Some(effective_package),
        |required_key| required_key == file_key,
    )
}

fn find_all_by_package(table: &Table, package: &str) -> Vec<(String, DependencySpec)> {
    table
        .iter()
        .filter_map(|(file_key, _)| {
            let mut spec = read_spec(table, file_key)?;
            let effective_package = spec.package.as_deref().unwrap_or(file_key);
            if effective_package == package {
                spec.package = Some(effective_package.to_owned());
                Some((file_key.to_owned(), spec))
            } else {
                None
            }
        })
        .collect()
}

fn effective_package<'a>(file_key: &'a str, spec: &'a DependencySpec) -> &'a str {
    spec.package.as_deref().unwrap_or(file_key)
}

fn spec_for_write_key(spec: &DependencySpec, write_key: &str) -> DependencySpec {
    let mut out = spec.clone();
    if out.package.as_deref() == Some(write_key) {
        out.package = None;
    }
    out
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
