//! TOML array item reconciliation.

use std::collections::{BTreeMap, BTreeSet};

use aqc_file_engine_core::{Finding, ResolvedItemRequirements};
use toml_edit::{Array, DocumentMut};

use crate::finding::{attribution, push_mismatch};
use crate::items::support;
use crate::items::types::{TomlArrayItem, TomlItemField};

/// Reconcile a TOML array field against resolved item requirements.
pub fn reconcile_array_items<ItemType>(
    doc: &mut DocumentMut,
    field: TomlItemField<'_>,
    requirements: &ResolvedItemRequirements<ItemType>,
    findings: &mut Vec<Finding>,
) where
    ItemType: TomlArrayItem,
    ItemType::Identity: ToString,
{
    if requirements.required.is_empty()
        && requirements.forbidden.is_empty()
        && requirements.exact.is_none()
    {
        return;
    }

    let exact_items_empty = requirements
        .exact
        .as_ref()
        .is_none_or(|exact| exact.items.is_empty());
    if requirements.required.is_empty()
        && exact_items_empty
        && support::array_item(doc, field).is_none()
    {
        return;
    }

    let malformed = support::report_array_shape(doc, field, requirements, findings);
    if malformed && requirements.required.is_empty() && requirements.exact.is_none() {
        return;
    }

    let array = support::ensure_array(doc, field);
    let current = collect_array_items::<ItemType>(array, field, requirements, findings);
    if !current.duplicate {
        apply_required_array_items(array, field, requirements, &current.positions, findings);
    }
    apply_forbidden_array_items(array, field, requirements, findings);
    if !current.duplicate {
        apply_exact_array_items(array, field, requirements, findings);
    }
}

fn collect_array_items<ItemType>(
    array: &Array,
    field: TomlItemField<'_>,
    requirements: &ResolvedItemRequirements<ItemType>,
    findings: &mut Vec<Finding>,
) -> support::CurrentItems<ItemType::Identity>
where
    ItemType: TomlArrayItem,
    ItemType::Identity: ToString,
{
    let mut out = BTreeMap::new();
    let mut duplicate = false;
    for (index, value) in array.iter().enumerate() {
        let current = match ItemType::read_value(value) {
            Ok(item) => item,
            Err(error) => {
                push_mismatch(
                    findings,
                    format!("{}[{index}]", field.display_key()),
                    Some(value.to_string()),
                    "valid item".to_owned(),
                    error.message().to_owned(),
                    &crate::items::support::item_attribution(requirements),
                );
                continue;
            }
        };
        let identity = current.merge_identity();
        if out.insert(identity.clone(), index).is_some() {
            duplicate = true;
            support::report_duplicate_identity(field, requirements, findings, &identity);
        }
    }
    support::CurrentItems {
        positions: out,
        duplicate,
    }
}

fn apply_required_array_items<ItemType>(
    array: &mut Array,
    field: TomlItemField<'_>,
    requirements: &ResolvedItemRequirements<ItemType>,
    current: &BTreeMap<ItemType::Identity, usize>,
    findings: &mut Vec<Finding>,
) where
    ItemType: TomlArrayItem,
    ItemType::Identity: ToString,
{
    let exact_items = requirements.exact.as_ref().map(|exact| &exact.items);
    for (identity, entry) in requirements.required.iter().chain(
        exact_items
            .into_iter()
            .flat_map(|items| items.iter())
            .filter(|(identity, _)| !requirements.required.contains_key(*identity)),
    ) {
        let attribution = attribution(entry);
        let key = support::item_key::<ItemType>(field, identity);
        let Some(index) = current.get(identity).copied() else {
            array.push(entry.merged.write_value());
            push_mismatch(
                findings,
                key,
                None,
                entry.merged.render_value(),
                support::item_message(&entry.collected),
                &attribution,
            );
            continue;
        };
        let Some(value) = array.get(index) else {
            continue;
        };
        let Ok(parsed) = ItemType::read_value(value) else {
            continue;
        };
        if ItemType::matches_value(&parsed, &entry.merged) && ItemType::is_canonical_value(value) {
            continue;
        }
        let current_rendered = Some(value.to_string());
        let _ = array.replace(index, entry.merged.write_value());
        push_mismatch(
            findings,
            key,
            current_rendered,
            entry.merged.render_value(),
            support::item_message(&entry.collected),
            &attribution,
        );
    }
}

fn apply_forbidden_array_items<ItemType>(
    array: &mut Array,
    field: TomlItemField<'_>,
    requirements: &ResolvedItemRequirements<ItemType>,
    findings: &mut Vec<Finding>,
) where
    ItemType: TomlArrayItem,
    ItemType::Identity: ToString,
{
    let forbidden = requirements
        .forbidden
        .keys()
        .cloned()
        .collect::<BTreeSet<_>>();
    for (index, identity) in support::remove_array_items(array, |value| {
        ItemType::read_value(value)
            .ok()
            .map(|item| item.merge_identity())
            .filter(|identity| forbidden.contains(identity))
    }) {
        let Some(entry) = requirements.forbidden.get(&identity) else {
            continue;
        };
        let _ = index;
        push_mismatch(
            findings,
            support::item_key::<ItemType>(field, &identity),
            Some(identity.to_string()),
            "absent".to_owned(),
            support::forbidden_message(&entry.collected),
            &attribution(entry),
        );
    }
}

fn apply_exact_array_items<ItemType>(
    array: &mut Array,
    field: TomlItemField<'_>,
    requirements: &ResolvedItemRequirements<ItemType>,
    findings: &mut Vec<Finding>,
) where
    ItemType: TomlArrayItem,
    ItemType::Identity: ToString,
{
    let Some(exact) = &requirements.exact else {
        return;
    };
    let allowed = &exact.identities;
    for (_, identity) in support::remove_array_items(array, |value| {
        ItemType::read_value(value)
            .ok()
            .map(|item| item.merge_identity())
            .filter(|identity| !allowed.contains(identity))
    }) {
        push_mismatch(
            findings,
            support::item_key::<ItemType>(field, &identity),
            Some(identity.to_string()),
            "absent (exact collection)".to_owned(),
            support::first_exact_message(requirements),
            &support::exact_attribution(requirements),
        );
    }
}
