//! Parse, compare, and render TOML dependency entries.

#![cfg_attr(
    not(test),
    expect(
        clippy::missing_docs_in_private_items,
        reason = "Private dependency TOML helpers are internal reconciliation steps."
    )
)]
#![expect(
    clippy::type_complexity,
    reason = "Dependency lookup helpers return Cargo file-key/spec pairs."
)]

use toml_edit::{Array, InlineTable, Item, Table, Value};

use crate::requirement::DependencySpec;

pub(super) fn find_all_by_package(table: &Table, package: &str) -> Vec<(String, DependencySpec)> {
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

pub(super) fn effective_package<'a>(file_key: &'a str, spec: &'a DependencySpec) -> &'a str {
    spec.package.as_deref().unwrap_or(file_key)
}

pub(super) fn spec_for_write_key(spec: &DependencySpec, write_key: &str) -> DependencySpec {
    let mut out = spec.clone();
    if out.package.as_deref() == Some(write_key) {
        out.package = None;
    }
    out
}

/// True when every field the spec sets equals the on-disk entry (partial match).
pub(super) fn spec_matches(spec: &DependencySpec, current: &DependencySpec) -> bool {
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

/// Read an existing dependency entry into a `DependencySpec`.
pub(super) fn read_spec(table: &Table, name: &str) -> Option<DependencySpec> {
    let item = table.get(name)?;
    if let Some(s) = item.as_str() {
        return Some(DependencySpec {
            version: Some(s.to_owned()),
            ..DependencySpec::default()
        });
    }
    if let Some(inline) = item.as_inline_table() {
        return Some(spec_from_inline(inline));
    }
    item.as_table().map(spec_from_table)
}

fn spec_from_inline(inline: &InlineTable) -> DependencySpec {
    let str_field = |k: &str| inline.get(k).and_then(Value::as_str).map(ToOwned::to_owned);
    DependencySpec {
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
    }
}

fn spec_from_table(table: &Table) -> DependencySpec {
    let str_field = |k: &str| table.get(k).and_then(Item::as_str).map(ToOwned::to_owned);
    DependencySpec {
        version: str_field("version"),
        features: table
            .get("features")
            .and_then(Item::as_array)
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(ToOwned::to_owned))
                    .collect()
            })
            .unwrap_or_default(),
        default_features: table.get("default-features").and_then(Item::as_bool),
        optional: table.get("optional").and_then(Item::as_bool),
        workspace: table.get("workspace").and_then(Item::as_bool),
        path: str_field("path"),
        git: str_field("git"),
        branch: str_field("branch"),
        tag: str_field("tag"),
        rev: str_field("rev"),
        registry: str_field("registry"),
        package: str_field("package"),
    }
}

/// Render a `DependencySpec` to write.
pub(super) fn spec_to_item(spec: &DependencySpec) -> Item {
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
