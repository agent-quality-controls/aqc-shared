//! Shared TOML table helpers.

use toml_edit::{ArrayOfTables, DocumentMut, Item, Table, TableLike};

/// Get a top-level table by key for reading.
#[must_use]
pub fn table_ref<'a>(doc: &'a DocumentMut, key: &str) -> Option<&'a dyn TableLike> {
    doc.get(key).and_then(Item::as_table_like)
}

/// Get or create a top-level table by key.
///
/// # Panics
///
/// Panics only if `toml_edit` returns a non-table for an item inserted as a table.
#[expect(
    clippy::expect_used,
    reason = "or_insert(Item::Table(_)) guarantees as_table_mut returns Some"
)]
pub fn ensure_table<'a>(doc: &'a mut DocumentMut, key: &str) -> &'a mut Table {
    doc.entry(key)
        .or_insert(Item::Table(Table::new()))
        .as_table_mut()
        .expect("entry just inserted as Table is a table")
}

/// Get or create a nested table under an existing table.
///
/// # Panics
///
/// Panics only if `toml_edit` returns a non-table for an item inserted as a table.
#[expect(
    clippy::expect_used,
    reason = "or_insert(Item::Table(_)) guarantees as_table_mut returns Some"
)]
pub fn ensure_nested<'a>(parent: &'a mut Table, key: &str) -> &'a mut Table {
    parent
        .entry(key)
        .or_insert(Item::Table(Table::new()))
        .as_table_mut()
        .expect("entry just inserted as Table is a table")
}

/// Read the table at a path.
#[must_use]
pub fn table_at<'a>(doc: &'a DocumentMut, path: &[String]) -> Option<&'a dyn TableLike> {
    let (first, rest) = path.split_first()?;
    let mut cur = doc.get(first)?.as_table_like()?;
    for segment in rest {
        cur = cur.get(segment)?.as_table_like()?;
    }
    Some(cur)
}

/// Read the table at a path mutably without creating it.
pub fn table_at_mut<'a>(doc: &'a mut DocumentMut, path: &[String]) -> Option<&'a mut Table> {
    let (first, rest) = path.split_first()?;
    let mut cur = doc.get_mut(first)?.as_table_mut()?;
    for segment in rest {
        cur = cur.get_mut(segment)?.as_table_mut()?;
    }
    Some(cur)
}

/// Get or create the table at a path.
///
/// # Panics
///
/// Panics if `path` is empty, or if `toml_edit` returns a non-table for an item
/// inserted as a table.
#[expect(
    clippy::expect_used,
    reason = "callers pass non-empty table paths and inserted tables remain tables"
)]
pub fn ensure_table_at<'a>(doc: &'a mut DocumentMut, path: &[String]) -> &'a mut Table {
    let (first, rest) = path.split_first().expect("table path is never empty");
    let mut cur = doc
        .entry(first)
        .or_insert(Item::Table(Table::new()))
        .as_table_mut()
        .expect("entry just inserted as Table is a table");
    for segment in rest {
        cur = cur
            .entry(segment)
            .or_insert(Item::Table(Table::new()))
            .as_table_mut()
            .expect("entry just inserted as Table is a table");
    }
    cur
}

/// Get or create an array-of-tables at a top-level key.
///
/// # Panics
///
/// Panics only if `toml_edit` returns a non-array-of-tables for an item inserted
/// as an array of tables.
#[expect(
    clippy::expect_used,
    reason = "or_insert(Item::ArrayOfTables(_)) guarantees as_array_of_tables_mut returns Some"
)]
pub fn ensure_array_of_tables<'a>(doc: &'a mut DocumentMut, key: &str) -> &'a mut ArrayOfTables {
    doc.entry(key)
        .or_insert(Item::ArrayOfTables(ArrayOfTables::new()))
        .as_array_of_tables_mut()
        .expect("entry just inserted as ArrayOfTables is an array of tables")
}
