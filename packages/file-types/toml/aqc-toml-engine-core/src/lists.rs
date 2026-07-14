//! Shared TOML list helpers.

use std::collections::BTreeSet;

use aqc_file_engine_core::{
    Finding, Provenance, ResolvedListRequirements, Severity, merge::Contributor,
};
use toml_edit::{Array, DocumentMut, Item, Table, TableLike, Value};

use crate::scalars::render_item;

/// Finding key shape for per-item list mismatches.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ListFieldKeyStyle {
    /// Report contains/excludes on the list field key.
    Field,
    /// Report contains/excludes on `<field>.<item>`.
    FieldItem,
}

/// Convert string values into a TOML array item.
#[must_use]
pub fn list_item(values: &[String]) -> Item {
    let mut array = Array::new();
    for value in values {
        array.push(Value::from(value.as_str()));
    }
    Item::Value(Value::Array(array))
}

/// Return true when a TOML item is an array containing the string value.
#[must_use]
pub fn list_contains(item: &Item, value: &str) -> bool {
    item.as_array()
        .is_some_and(|array| array.iter().any(|current| current.as_str() == Some(value)))
}

/// Read string values from a TOML array, dropping malformed entries.
#[must_use]
pub fn list_values(doc: &DocumentMut, key: &str) -> Vec<String> {
    doc.get(key)
        .and_then(Item::as_array)
        .map(|array| {
            array
                .iter()
                .filter_map(|value| value.as_str().map(ToOwned::to_owned))
                .collect()
        })
        .unwrap_or_default()
}

/// Write a TOML string array to one top-level key.
pub fn write_list(doc: &mut DocumentMut, key: &str, values: &[String]) {
    doc[key] = list_item(values);
}

/// Read string values from a TOML table array field, dropping malformed entries.
#[must_use]
pub fn table_list_values(table: &dyn TableLike, key: &str) -> Vec<String> {
    table_list_values_optional(table, key).unwrap_or_default()
}

/// Read string values from a TOML table array field when it exists and is an array.
#[must_use]
pub fn table_list_values_optional(table: &dyn TableLike, key: &str) -> Option<Vec<String>> {
    table.get(key).and_then(Item::as_array).map(|array| {
        array
            .iter()
            .filter_map(|value| value.as_str().map(ToOwned::to_owned))
            .collect()
    })
}

/// Write a TOML string array to one table field.
pub fn write_table_list(table: &mut Table, key: &str, values: &[String]) {
    table[key] = list_item(values);
}

/// Reconcile a table string-array field and return the replacement list.
pub fn reconcile_table_list_field(
    display_key: String,
    current: Vec<String>,
    requirements: &ResolvedListRequirements,
    findings: &mut Vec<Finding>,
) -> Option<Vec<String>> {
    reconcile_list_field(
        display_key,
        current,
        requirements,
        ListFieldKeyStyle::Field,
        findings,
    )
}

/// Reconcile a string-list field and return the replacement list.
pub fn reconcile_list_field(
    display_key: String,
    current: Vec<String>,
    requirements: &ResolvedListRequirements,
    key_style: ListFieldKeyStyle,
    findings: &mut Vec<Finding>,
) -> Option<Vec<String>> {
    let mut changed = false;
    let mut out = current;

    for (item, entry) in &requirements.contains {
        let attribution = entry.attribution();
        let msg = first_item_message(&entry.collected);
        let present: BTreeSet<&str> = out.iter().map(String::as_str).collect();
        if present.contains(item.as_str()) {
            continue;
        }
        let key = list_item_key(&display_key, item, key_style);
        findings.push(Finding::Mismatch {
            key,
            selector: None,
            current: Some(format!("{out:?}")),
            expected: format!("contains {item:?}"),
            message: msg,
            severity: Severity::Error,
            attribution,
        });
        out.push(item.clone());
        changed = true;
    }

    for (item, entry) in &requirements.excludes {
        if !out.contains(item) {
            continue;
        }
        let attribution = entry.attribution();
        let msg = first_item_message(&entry.collected);
        let key = list_item_key(&display_key, item, key_style);
        findings.push(Finding::Mismatch {
            key,
            selector: None,
            current: Some(format!("{out:?}")),
            expected: format!("excludes {item:?}"),
            message: msg,
            severity: Severity::Error,
            attribution,
        });
        out.retain(|value| value != item);
        changed = true;
    }

    if let Some(exact) = &requirements.exact {
        let expected = exact.merged.clone();
        if out != expected {
            let attribution = exact.attribution();
            let msg = exact
                .collected
                .first()
                .map(|(_, (_, msg))| msg.clone())
                .unwrap_or_default();
            findings.push(Finding::Mismatch {
                key: display_key,
                selector: None,
                current: Some(format!("{out:?}")),
                expected: format!("{expected:?}"),
                message: msg,
                severity: Severity::Error,
                attribution,
            });
            out = expected;
            changed = true;
        }
    }

    changed.then_some(out)
}

/// Builds the mismatch finding key for one list item.
fn list_item_key(display_key: &str, item: &str, key_style: ListFieldKeyStyle) -> String {
    match key_style {
        ListFieldKeyStyle::Field => display_key.to_owned(),
        ListFieldKeyStyle::FieldItem => format!("{display_key}.{item}"),
    }
}

/// Returns the first user-facing message collected for one list item.
fn first_item_message(collected: &[Contributor]) -> String {
    collected
        .first()
        .map(|(_, msg)| msg.clone())
        .unwrap_or_default()
}

/// Render an existing list item or an empty-list placeholder.
#[must_use]
pub fn render_list(doc: &DocumentMut, key: &str) -> String {
    doc.get(key)
        .and_then(render_item)
        .unwrap_or_else(|| "[]".to_owned())
}

/// Return the first message attached to a list requirement.
#[must_use]
pub fn list_message(requirements: &ResolvedListRequirements) -> String {
    requirements
        .contains
        .values()
        .flat_map(|resolved| resolved.collected.iter().map(|(_, msg)| msg.as_str()))
        .chain(
            requirements
                .excludes
                .values()
                .flat_map(|resolved| resolved.collected.iter().map(|(_, msg)| msg.as_str())),
        )
        .chain(
            requirements
                .exact
                .iter()
                .flat_map(|resolved| resolved.collected.iter().map(|(_, (_, msg))| msg.as_str())),
        )
        .next()
        .unwrap_or_default()
        .to_owned()
}

/// Report malformed list shape before applying list requirements.
pub fn report_list_shape(
    doc: &DocumentMut,
    key: &str,
    requirements: &ResolvedListRequirements,
    findings: &mut Vec<Finding>,
) -> bool {
    let attr = list_attribution(requirements);
    report_list_shape_with_message(doc, key, list_message(requirements), &attr, findings)
}

/// Report malformed list shape before applying non-list-backed requirements.
pub fn report_list_shape_with_message(
    doc: &DocumentMut,
    key: &str,
    message: String,
    attr: &[Provenance],
    findings: &mut Vec<Finding>,
) -> bool {
    let Some(item) = doc.get(key) else {
        return false;
    };
    let Some(array) = item.as_array() else {
        findings.push(Finding::Mismatch {
            key: key.to_owned(),
            selector: None,
            current: render_item(item),
            expected: "array of strings".to_owned(),
            message,
            severity: Severity::Error,
            attribution: attr.to_vec(),
        });
        return true;
    };
    let mut malformed = false;
    for (index, value) in array.iter().enumerate() {
        if value.as_str().is_some() {
            continue;
        }
        malformed = true;
        findings.push(Finding::Mismatch {
            key: format!("{key}[{index}]"),
            selector: None,
            current: Some(value.to_string()),
            expected: "string".to_owned(),
            message: message.clone(),
            severity: Severity::Error,
            attribution: attr.to_vec(),
        });
    }
    malformed
}

/// Returns all policy provenance attached to a resolved list requirement.
fn list_attribution(requirements: &ResolvedListRequirements) -> Vec<Provenance> {
    requirements
        .contains
        .values()
        .flat_map(aqc_file_engine_core::ResolvedRequirement::attribution)
        .chain(
            requirements
                .excludes
                .values()
                .flat_map(aqc_file_engine_core::ResolvedRequirement::attribution),
        )
        .chain(
            requirements
                .exact
                .iter()
                .flat_map(aqc_file_engine_core::ResolvedRequirement::attribution),
        )
        .collect()
}
