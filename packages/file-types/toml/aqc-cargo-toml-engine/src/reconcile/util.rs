//! Cargo-only TOML helpers for reconciliation.

use toml_edit::{InlineTable, Item, Value};

/// The inline-table inheritance form `{ workspace = true }`.
pub(crate) fn workspace_inline() -> Item {
    let mut table = InlineTable::new();
    let _ = table.insert("workspace", Value::from(true));
    Item::Value(Value::InlineTable(table))
}

/// True when the on-disk item is the workspace inheritance form.
pub(crate) fn is_workspace_inherit(item: &Item) -> bool {
    if let Some(table) = item.as_inline_table() {
        return table.get("workspace").and_then(Value::as_bool) == Some(true);
    }
    if let Some(table) = item.as_table() {
        return table.get("workspace").and_then(Item::as_bool) == Some(true);
    }
    false
}
