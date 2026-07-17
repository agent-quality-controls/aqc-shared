//! Shared support for TOML item reconciliation.

#![allow(
    clippy::expect_used,
    reason = "These expects guard internal post-conditions immediately after replacing TOML item variants."
)]

use std::collections::BTreeMap;

use aqc_file_engine_core::{
    FileItemRequirement, Finding, Provenance, ResolvedItemRequirements, Severity,
};
use toml_edit::{Array, ArrayOfTables, DocumentMut, Item, Table, TableLike, Value};

use crate::items::types::TomlItemField;
use crate::scalars::render_item;
use crate::tables::{ensure_array_of_tables, ensure_table_at, table_at_mut};

pub(super) struct CurrentItems<Identity> {
    pub(super) positions: BTreeMap<Identity, usize>,
    pub(super) duplicate: bool,
}

pub(super) fn array_item<'a>(doc: &'a DocumentMut, field: TomlItemField<'_>) -> Option<&'a Item> {
    if field.table_path().is_empty() {
        doc.get(field.field_key())
    } else {
        let path = field
            .table_path()
            .iter()
            .map(|segment| (*segment).to_owned())
            .collect::<Vec<_>>();
        aqc_toml_path_ref(doc, &path).and_then(|table| table.get(field.field_key()))
    }
}

pub(super) fn ensure_array<'a>(
    doc: &'a mut DocumentMut,
    field: TomlItemField<'_>,
) -> &'a mut Array {
    let item = if field.table_path().is_empty() {
        doc.entry(field.field_key())
            .or_insert(Item::Value(Value::Array(Array::new())))
    } else {
        let path = field
            .table_path()
            .iter()
            .map(|segment| (*segment).to_owned())
            .collect::<Vec<_>>();
        ensure_table_at(doc, &path)
            .entry(field.field_key())
            .or_insert(Item::Value(Value::Array(Array::new())))
    };
    if !item.is_array() {
        *item = Item::Value(Value::Array(Array::new()));
    }
    item.as_array_mut()
        .expect("entry just inserted as Array is an array")
}

pub(super) fn ensure_array_table<'a>(
    doc: &'a mut DocumentMut,
    field: TomlItemField<'_>,
) -> &'a mut ArrayOfTables {
    if field.table_path().is_empty() {
        return ensure_array_of_tables(doc, field.field_key());
    }
    let path = field
        .table_path()
        .iter()
        .map(|segment| (*segment).to_owned())
        .collect::<Vec<_>>();
    if table_at_mut(doc, &path).is_none() {
        let _ = ensure_table_at(doc, &path);
    }
    let item = table_at_mut(doc, &path)
        .expect("table path was just created")
        .entry(field.field_key())
        .or_insert(Item::ArrayOfTables(ArrayOfTables::new()));
    if !item.is_array_of_tables() {
        *item = Item::ArrayOfTables(ArrayOfTables::new());
    }
    item.as_array_of_tables_mut()
        .expect("entry just inserted as ArrayOfTables is an array of tables")
}

pub(super) fn report_array_shape<ItemType>(
    doc: &DocumentMut,
    field: TomlItemField<'_>,
    requirements: &ResolvedItemRequirements<ItemType>,
    findings: &mut Vec<Finding>,
) -> bool
where
    ItemType: FileItemRequirement,
{
    let Some(item) = array_item(doc, field) else {
        return false;
    };
    if item.is_array() {
        return false;
    }
    findings.push(Finding::Mismatch {
        key: field.display_key().to_owned(),
        selector: None,
        current: render_item(item),
        expected: "array".to_owned(),
        message: first_message(requirements),
        severity: Severity::Error,
        attribution: item_attribution(requirements),
    });
    true
}

pub(super) fn report_duplicate_identity<ItemType>(
    field: TomlItemField<'_>,
    requirements: &ResolvedItemRequirements<ItemType>,
    findings: &mut Vec<Finding>,
    identity: &ItemType::Identity,
) where
    ItemType: FileItemRequirement,
    ItemType::Identity: ToString,
{
    if requirements.required.is_empty()
        && requirements.allowed.is_none()
        && requirements.exact.is_none()
    {
        return;
    }
    findings.push(Finding::InvalidRequirements {
        key: format!("{}.{}", field.display_key(), identity.to_string()),
        message: "duplicate item identity".to_owned(),
        contributors: item_attribution(requirements)
            .into_iter()
            .map(|prov| (prov.policy, "duplicate item identity".to_owned()))
            .collect(),
    });
}

pub(super) fn remove_array_items<Identity>(
    array: &mut Array,
    identity: impl Fn(&Value) -> Option<Identity>,
) -> Vec<(usize, Identity)> {
    let mut removals = Vec::new();
    for (index, value) in array.iter().enumerate() {
        if let Some(found) = identity(value) {
            removals.push((index, found));
        }
    }
    for (index, _) in removals.iter().rev() {
        let _ = array.remove(*index);
    }
    removals
}

pub(super) fn remove_array_table_items<Identity>(
    array: &mut ArrayOfTables,
    identity: impl Fn(&Table) -> Option<Identity>,
) -> Vec<Identity> {
    let mut removals = Vec::new();
    for (index, table) in array.iter().enumerate() {
        if let Some(found) = identity(table) {
            removals.push((index, found));
        }
    }
    for (index, _) in removals.iter().rev() {
        let _ = array.remove(*index);
    }
    removals
        .into_iter()
        .map(|(_, removed_identity)| removed_identity)
        .collect()
}

pub(super) fn item_key<ItemType>(field: TomlItemField<'_>, identity: &ItemType::Identity) -> String
where
    ItemType: FileItemRequirement,
    ItemType::Identity: ToString,
{
    format!("{}.{}", field.display_key(), identity.to_string())
}

pub(super) fn item_message<ItemType>(collected: &[(Provenance, (ItemType, String))]) -> String {
    collected
        .first()
        .map(|(_, (_, msg))| msg.clone())
        .unwrap_or_default()
}

pub(super) fn forbidden_message(collected: &[(Provenance, String)]) -> String {
    collected
        .first()
        .map(|(_, msg)| msg.clone())
        .unwrap_or_default()
}

pub(super) fn item_attribution<ItemType>(
    requirements: &ResolvedItemRequirements<ItemType>,
) -> Vec<Provenance>
where
    ItemType: FileItemRequirement,
{
    requirements
        .required
        .values()
        .flat_map(aqc_file_engine_core::ResolvedRequirement::attribution)
        .chain(
            requirements
                .forbidden
                .values()
                .flat_map(aqc_file_engine_core::ResolvedRequirement::attribution),
        )
        .chain(
            requirements
                .membership()
                .into_iter()
                .flat_map(|membership| membership.all_attribution()),
        )
        .collect()
}

fn aqc_toml_path_ref<'a>(doc: &'a DocumentMut, path: &[String]) -> Option<&'a dyn TableLike> {
    let (first, rest) = path.split_first()?;
    let mut cur = doc.get(first)?.as_table_like()?;
    for segment in rest {
        cur = cur.get(segment)?.as_table_like()?;
    }
    Some(cur)
}

fn first_message<ItemType>(requirements: &ResolvedItemRequirements<ItemType>) -> String
where
    ItemType: FileItemRequirement,
{
    requirements
        .required
        .values()
        .flat_map(|resolved| resolved.collected.iter().map(|(_, (_, msg))| msg.as_str()))
        .chain(
            requirements
                .allowed
                .iter()
                .flat_map(|allowed| allowed.collected.iter().map(|(_, (_, msg))| msg.as_str())),
        )
        .chain(
            requirements
                .forbidden
                .values()
                .flat_map(|resolved| resolved.collected.iter().map(|(_, msg)| msg.as_str())),
        )
        .chain(
            requirements
                .exact
                .iter()
                .flat_map(|exact| exact.collected.iter().map(|(_, (_, msg))| msg.as_str())),
        )
        .next()
        .unwrap_or_default()
        .to_owned()
}
