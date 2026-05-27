//! Helpers shared across reconcile submodules.

use aqc_file_engine_core::{MergedAssertion, Provenance};
use toml_edit::{DocumentMut, Item, Table};

/// Collect provenances of all contributions in a `MergedAssertion`.
pub(crate) fn all_provenances<A>(merged: &MergedAssertion<A>) -> Vec<Provenance> {
    merged
        .contributions
        .iter()
        .map(|(p, _)| p.clone())
        .collect()
}

/// Get or create a top-level table in the document, returning a mutable reference.
#[expect(
    clippy::expect_used,
    reason = "or_insert(Item::Table(_)) guarantees as_table_mut returns Some"
)]
pub(crate) fn get_or_create_table_mut<'a>(doc: &'a mut DocumentMut, key: &str) -> &'a mut Table {
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
pub(crate) fn get_or_create_nested_table_mut<'a>(
    parent: &'a mut Table,
    key: &str,
) -> &'a mut Table {
    parent
        .entry(key)
        .or_insert(Item::Table(Table::new()))
        .as_table_mut()
        .expect("entry just inserted as Table is a table")
}
