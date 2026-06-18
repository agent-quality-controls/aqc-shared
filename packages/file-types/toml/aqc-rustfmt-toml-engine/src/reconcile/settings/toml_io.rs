//! Shared TOML helpers for settings reconciliation.

use aqc_file_engine_core::{Provenance, ResolvedRequirement};
use toml_edit::{Array, DocumentMut, Item, Value, value as toml_value};

/// Renders a TOML item when it is a value.
pub(super) fn render_item(item: &Item) -> Option<String> {
    item.as_value().map(ToString::to_string)
}

/// Reads string values from a TOML array, dropping malformed entries.
pub(super) fn list_values(doc: &DocumentMut, key: &str) -> Vec<String> {
    doc.get(key)
        .and_then(Item::as_array)
        .map(|array| {
            array
                .iter()
                .filter_map(Value::as_str)
                .map(ToOwned::to_owned)
                .collect()
        })
        .unwrap_or_default()
}

/// Writes a string array to a TOML key.
pub(super) fn write_list(doc: &mut DocumentMut, key: &str, values: &[String]) {
    let mut array = Array::default();
    for item in values {
        array.push(item.as_str());
    }
    doc[key] = toml_value(array);
}

/// Renders an existing list value or an empty-list placeholder.
pub(super) fn render_list(doc: &DocumentMut, key: &str) -> String {
    doc.get(key)
        .and_then(render_item)
        .unwrap_or_else(|| "[]".to_owned())
}

/// Extracts provenance from a resolved requirement.
pub(super) fn attribution<Merged, Assertion>(
    resolved: &ResolvedRequirement<Merged, Assertion>,
) -> Vec<Provenance> {
    resolved
        .collected
        .iter()
        .map(|(prov, _)| prov.clone())
        .collect()
}
