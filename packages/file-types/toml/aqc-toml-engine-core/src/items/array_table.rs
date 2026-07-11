//! TOML array-of-tables item reconciliation.

use std::collections::{BTreeMap, BTreeSet};

use aqc_file_engine_core::{Finding, ResolvedItemRequirements};
use toml_edit::{ArrayOfTables, DocumentMut};

use crate::finding::{attribution, push_mismatch};
use crate::items::support::{
    CurrentItems, array_item, ensure_array_table, exact_attribution, first_exact_message,
    forbidden_message, item_key, item_message, remove_array_table_items, report_duplicate_identity,
};
use crate::items::types::{TomlArrayTableItem, TomlItemField};

/// Reconcile a TOML array-of-tables field against resolved item requirements.
pub fn reconcile_array_table_items<ItemType>(
    doc: &mut DocumentMut,
    field: TomlItemField<'_>,
    requirements: &ResolvedItemRequirements<ItemType>,
    findings: &mut Vec<Finding>,
) where
    ItemType: TomlArrayTableItem,
    ItemType::Identity: ToString,
{
    if requirements.required.is_empty()
        && requirements.forbidden.is_empty()
        && requirements.exact.is_none()
    {
        return;
    }

    if requirements.required.is_empty() && array_item(doc, field).is_none() {
        return;
    }

    let array = ensure_array_table(doc, field);
    let current = collect_array_table_items::<ItemType>(array, field, requirements, findings);
    if !current.duplicate {
        apply_required_array_table_items(array, field, requirements, &current.positions, findings);
    }
    apply_forbidden_array_table_items(array, field, requirements, findings);
    if !current.duplicate {
        apply_exact_array_table_items(array, field, requirements, findings);
    }
}

fn collect_array_table_items<ItemType>(
    array: &ArrayOfTables,
    field: TomlItemField<'_>,
    requirements: &ResolvedItemRequirements<ItemType>,
    findings: &mut Vec<Finding>,
) -> CurrentItems<ItemType::Identity>
where
    ItemType: TomlArrayTableItem,
    ItemType::Identity: ToString,
{
    let mut out = BTreeMap::new();
    let mut duplicate = false;
    for (index, table) in array.iter().enumerate() {
        let current = match ItemType::read_table(table) {
            Ok(item) => item,
            Err(error) => {
                push_mismatch(
                    findings,
                    format!("{}[{index}]", field.display_key()),
                    Some(table.to_string()),
                    "valid table item".to_owned(),
                    error.message().to_owned(),
                    &crate::items::support::item_attribution(requirements),
                );
                continue;
            }
        };
        let identity = current.merge_identity();
        if out.insert(identity.clone(), index).is_some() {
            duplicate = true;
            report_duplicate_identity(field, requirements, findings, &identity);
        }
    }
    CurrentItems {
        positions: out,
        duplicate,
    }
}

fn apply_required_array_table_items<ItemType>(
    array: &mut ArrayOfTables,
    field: TomlItemField<'_>,
    requirements: &ResolvedItemRequirements<ItemType>,
    current: &BTreeMap<ItemType::Identity, usize>,
    findings: &mut Vec<Finding>,
) where
    ItemType: TomlArrayTableItem,
    ItemType::Identity: ToString,
{
    for (identity, entry) in &requirements.required {
        let attribution = attribution(entry);
        let key = item_key::<ItemType>(field, identity);
        let Some(index) = current.get(identity).copied() else {
            array.push(entry.merged.write_table());
            push_mismatch(
                findings,
                key,
                None,
                entry.merged.render_table(),
                item_message(&entry.collected),
                &attribution,
            );
            continue;
        };
        let Some(table) = array.get(index) else {
            continue;
        };
        let Ok(parsed) = ItemType::read_table(table) else {
            continue;
        };
        if ItemType::matches_table(&parsed, &entry.merged) {
            continue;
        }
        let current_rendered = Some(table.to_string());
        let _ = array.replace(index, entry.merged.write_table());
        push_mismatch(
            findings,
            key,
            current_rendered,
            entry.merged.render_table(),
            item_message(&entry.collected),
            &attribution,
        );
    }
}

fn apply_forbidden_array_table_items<ItemType>(
    array: &mut ArrayOfTables,
    field: TomlItemField<'_>,
    requirements: &ResolvedItemRequirements<ItemType>,
    findings: &mut Vec<Finding>,
) where
    ItemType: TomlArrayTableItem,
    ItemType::Identity: ToString,
{
    let forbidden = requirements
        .forbidden
        .keys()
        .cloned()
        .collect::<BTreeSet<_>>();
    for identity in remove_array_table_items(array, |table| {
        ItemType::read_table(table)
            .ok()
            .map(|item| item.merge_identity())
            .filter(|identity| forbidden.contains(identity))
    }) {
        let Some(entry) = requirements.forbidden.get(&identity) else {
            continue;
        };
        push_mismatch(
            findings,
            item_key::<ItemType>(field, &identity),
            Some(identity.to_string()),
            "absent".to_owned(),
            forbidden_message(&entry.collected),
            &attribution(entry),
        );
    }
}

fn apply_exact_array_table_items<ItemType>(
    array: &mut ArrayOfTables,
    field: TomlItemField<'_>,
    requirements: &ResolvedItemRequirements<ItemType>,
    findings: &mut Vec<Finding>,
) where
    ItemType: TomlArrayTableItem,
    ItemType::Identity: ToString,
{
    let Some(exact) = &requirements.exact else {
        return;
    };
    let allowed = &exact.identities;
    for identity in remove_array_table_items(array, |table| {
        ItemType::read_table(table)
            .ok()
            .map(|item| item.merge_identity())
            .filter(|identity| !allowed.contains(identity))
    }) {
        push_mismatch(
            findings,
            item_key::<ItemType>(field, &identity),
            Some(identity.to_string()),
            "absent (exact collection)".to_owned(),
            first_exact_message(requirements),
            &exact_attribution(requirements),
        );
    }
}
