//! Helpers shared across reconcile submodules.
//!
//! The cross-cutting rule these helpers serve: a check-only assertion must
//! never mutate the document, so tables are created lazily (only on an actual
//! write). Reads go through `doc.get(..)` / `table.get(..)`; the
//! `ensure_*` helpers create a table only when a write is about to happen.

#![expect(
    clippy::type_complexity,
    reason = "Collected assertions are plainly Vec<(Provenance, A)> and per-key maps of them; the shapes are declared openly at every signature instead of hidden behind wrapper types or aliases (taxonomy decision 2026-06-07)."
)]
use aqc_file_engine_core::{ConfigScalar, Finding, Provenance, Severity, parse_version_tuple};
use toml_edit::{Array, ArrayOfTables, DocumentMut, InlineTable, Item, Table, Value, value};

/// Collect the provenances of a collected-assertion list.
pub(crate) fn all_provenances<A>(pairs: &[(Provenance, A)]) -> Vec<Provenance> {
    pairs.iter().map(|(p, _)| p.clone()).collect()
}

/// Push a writable-key `Mismatch` finding (Error severity).
pub(crate) fn push_mismatch(
    findings: &mut Vec<Finding>,
    key: String,
    current: Option<String>,
    expected: String,
    message: String,
    attribution: &[Provenance],
) {
    findings.push(Finding::Mismatch {
        key,
        current,
        expected,
        message,
        severity: Severity::Error,
        attribution: attribution.to_vec(),
    });
}

/// Get a top-level table by key for reading, if it exists as a table.
pub(crate) fn table_ref<'a>(doc: &'a DocumentMut, key: &str) -> Option<&'a Table> {
    doc.get(key).and_then(Item::as_table)
}

/// Get or create a top-level table, returning a mutable reference. Call only
/// when a write is about to happen.
#[expect(
    clippy::expect_used,
    reason = "or_insert(Item::Table(_)) guarantees as_table_mut returns Some"
)]
pub(crate) fn ensure_table<'a>(doc: &'a mut DocumentMut, key: &str) -> &'a mut Table {
    doc.entry(key)
        .or_insert(Item::Table(Table::new()))
        .as_table_mut()
        .expect("entry just inserted as Table is a table")
}

/// Get or create a nested table `parent.child`, returning a mutable reference.
#[expect(
    clippy::expect_used,
    reason = "or_insert(Item::Table(_)) guarantees as_table_mut returns Some"
)]
pub(crate) fn ensure_nested<'a>(parent: &'a mut Table, key: &str) -> &'a mut Table {
    parent
        .entry(key)
        .or_insert(Item::Table(Table::new()))
        .as_table_mut()
        .expect("entry just inserted as Table is a table")
}

/// Convert a `ConfigScalar` to a `toml_edit::Item` for writing.
pub(crate) fn scalar_item(scalar: &ConfigScalar) -> Item {
    match scalar {
        ConfigScalar::Str(s) => value(s.clone()),
        ConfigScalar::Int(i) => value(*i),
        ConfigScalar::Bool(b) => value(*b),
    }
}

/// True when the on-disk item equals the scalar. A bare string entry compares
/// against `Str`; integers and bools compare against their forms.
pub(crate) fn scalar_matches(item: &Item, scalar: &ConfigScalar) -> bool {
    match scalar {
        ConfigScalar::Str(s) => item.as_str() == Some(s.as_str()),
        ConfigScalar::Int(i) => item.as_integer() == Some(*i),
        ConfigScalar::Bool(b) => item.as_bool() == Some(*b),
    }
}

/// Render a `ConfigScalar` for a finding's `expected` slot.
pub(crate) fn render_scalar(scalar: &ConfigScalar) -> String {
    match scalar {
        ConfigScalar::Str(s) => s.clone(),
        ConfigScalar::Int(i) => i.to_string(),
        ConfigScalar::Bool(b) => b.to_string(),
    }
}

/// Render an on-disk scalar item for a finding's `current` slot, if scalar.
pub(crate) fn render_item(item: &Item) -> Option<String> {
    item.as_value().map(|v| v.to_string().trim().to_owned())
}

/// True when the on-disk version `current` is at least `min`. Works for
/// semver-ish strings and editions (`2021`), via `parse_version_tuple`.
pub(crate) fn ge_version(current: &str, min: &str) -> bool {
    parse_version_tuple(current) >= parse_version_tuple(min)
}

/// Read an on-disk array of strings for `field` in `table`. Empty when absent
/// or not an array.
pub(crate) fn read_string_array(table: &Table, field: &str) -> Vec<String> {
    let Some(arr) = table.get(field).and_then(Item::as_array) else {
        return Vec::new();
    };
    arr.iter()
        .filter_map(|v| v.as_str().map(ToOwned::to_owned))
        .collect()
}

/// Write `field = ["a", "b", ...]` into `table`.
pub(crate) fn write_string_array(table: &mut Table, field: &str, items: &[String]) {
    let mut arr = Array::new();
    for it in items {
        arr.push(Value::from(it.as_str()));
    }
    table[field] = Item::Value(Value::Array(arr));
}

/// The inline-table inheritance form `{ workspace = true }`.
pub(crate) fn workspace_inline() -> Item {
    let mut t = InlineTable::new();
    let _ = t.insert("workspace", Value::from(true));
    Item::Value(Value::InlineTable(t))
}

/// True when the on-disk item is the inheritance form (`{ workspace = true }`
/// inline or `field.workspace = true` sub-table).
pub(crate) fn is_workspace_inherit(item: &Item) -> bool {
    if let Some(t) = item.as_inline_table() {
        return t.get("workspace").and_then(Value::as_bool) == Some(true);
    }
    if let Some(t) = item.as_table() {
        return t.get("workspace").and_then(Item::as_bool) == Some(true);
    }
    false
}

/// Read the table at `path`, if every segment exists as a table.
pub(crate) fn table_at<'a>(doc: &'a DocumentMut, path: &[String]) -> Option<&'a Table> {
    let (first, rest) = path.split_first()?;
    let mut cur: &Table = doc.get(first)?.as_table()?;
    for seg in rest {
        cur = cur.get(seg)?.as_table()?;
    }
    Some(cur)
}

/// Read the table at `path` mutably if it already exists (removals only;
/// never creates).
pub(crate) fn table_at_mut<'a>(doc: &'a mut DocumentMut, path: &[String]) -> Option<&'a mut Table> {
    let (first, rest) = path.split_first()?;
    let mut cur: &mut Table = doc.get_mut(first)?.as_table_mut()?;
    for seg in rest {
        cur = cur.get_mut(seg)?.as_table_mut()?;
    }
    Some(cur)
}

/// Get or create the table at `path`, creating each segment lazily. Call only
/// when a write is about to happen.
#[expect(
    clippy::expect_used,
    reason = "every caller passes a non-empty literal path, and or_insert(Item::Table(_)) guarantees as_table_mut returns Some at each level"
)]
pub(crate) fn ensure_table_at<'a>(doc: &'a mut DocumentMut, path: &[String]) -> &'a mut Table {
    let (first, rest) = path.split_first().expect("table path is never empty");
    let mut cur: &mut Table = doc
        .entry(first)
        .or_insert(Item::Table(Table::new()))
        .as_table_mut()
        .expect("entry just inserted as Table is a table");
    for seg in rest {
        cur = cur
            .entry(seg)
            .or_insert(Item::Table(Table::new()))
            .as_table_mut()
            .expect("entry just inserted as Table is a table");
    }
    cur
}

/// Get a mutable array-of-tables at top-level `key`, creating it on write.
#[expect(
    clippy::expect_used,
    reason = "or_insert(Item::ArrayOfTables(_)) guarantees as_array_of_tables_mut returns Some"
)]
pub(crate) fn ensure_array_of_tables<'a>(
    doc: &'a mut DocumentMut,
    key: &str,
) -> &'a mut ArrayOfTables {
    doc.entry(key)
        .or_insert(Item::ArrayOfTables(ArrayOfTables::new()))
        .as_array_of_tables_mut()
        .expect("entry just inserted as ArrayOfTables is an array of tables")
}
