//! Item and forbidden-glob merge functions.

use std::collections::BTreeSet;

use super::{
    ClosedInput, ConflictEntry, Contributor, FileItemRequirement, ForbiddenGlobRequirement,
    ForbiddenItemMap, GlobAssertionGroups, GlobInput, GlobResolutionMap, ItemAssertionGroups,
    ItemAssertionInput, ItemInput, ItemRequirementMap, KeyedItem, RequiredItemResolution,
    ResolvedForbiddenGlobRequirements, ResolvedItemRequirements, ResolvedRequirement,
};
use crate::types::Provenance;

pub fn resolve_items<Item>(
    key: &str,
    input: Vec<ItemInput<Item>>,
    conflicts: &mut Vec<ConflictEntry>,
) -> ResolvedItemRequirements<Item>
where
    Item: FileItemRequirement,
    Item::Identity: ToString,
{
    let mut grouped = ItemGroups::default();
    collect_item_groups(input, &mut grouped);

    let resolved_required = resolve_required_items(key, grouped.required, conflicts);
    let resolved_forbidden = resolve_forbidden_items(grouped.forbidden);

    report_required_forbidden_conflicts(key, &resolved_required, &resolved_forbidden, conflicts);
    report_closed_collection_conflicts(key, &grouped.closed_inputs, &resolved_required, conflicts);

    ResolvedItemRequirements {
        required: resolved_required,
        forbidden: resolved_forbidden,
        closed_by: grouped.closed_by,
    }
}

pub fn resolve_forbidden_globs<Glob>(
    _key: &str,
    input: Vec<GlobInput<Glob>>,
    conflicts: &mut Vec<ConflictEntry>,
) -> ResolvedForbiddenGlobRequirements<Glob>
where
    Glob: ForbiddenGlobRequirement,
    Glob::Identity: ToString,
{
    conflicts.reserve(0);
    let mut by_glob = GlobAssertionGroups::<Glob>::new();

    for (prov, globs) in input {
        for (glob, msg) in globs.globs {
            by_glob
                .entry(glob.merge_identity())
                .or_default()
                .push((prov.clone(), (glob, msg)));
        }
    }

    let mut resolved_globs = GlobResolutionMap::<Glob>::new();
    for (identity, items) in by_glob {
        let Some((_, (first, _))) = items.first() else {
            continue;
        };
        let _ = resolved_globs.insert(
            identity,
            ResolvedRequirement {
                merged: first.clone(),
                collected: items
                    .into_iter()
                    .map(|(prov, (_, msg))| (prov, msg))
                    .collect(),
            },
        );
    }

    ResolvedForbiddenGlobRequirements {
        globs: resolved_globs,
    }
}

pub fn compose_item_by<Item, Semantic>(
    key: &str,
    items: Vec<ItemAssertionInput<Item>>,
    project: impl Fn(&Item) -> Semantic,
    conflicts: &mut Vec<ConflictEntry>,
) -> Option<RequiredItemResolution<Item>>
where
    Item: Clone,
    Semantic: PartialEq,
{
    let mut iter = items.iter();
    let (_, (first, _)) = iter.next()?;
    let first_semantic = project(first);
    let disagree = iter.any(|(_, (entry, _))| project(entry) != first_semantic);
    if disagree {
        conflicts.push(ConflictEntry {
            key: key.to_owned(),
            reason: "set-key-disagree".to_owned(),
            contributors: items
                .iter()
                .map(|(prov, _)| required_contributor(prov))
                .collect(),
        });
        None
    } else {
        Some(ResolvedRequirement {
            merged: first.clone(),
            collected: items,
        })
    }
}

impl<Value> FileItemRequirement for KeyedItem<Value>
where
    Value: PartialEq + Clone,
{
    type Identity = String;

    fn merge_identity(&self) -> Self::Identity {
        self.file_key.clone()
    }

    fn compose_item(
        key: &str,
        items: Vec<ItemAssertionInput<Self>>,
        conflicts: &mut Vec<ConflictEntry>,
    ) -> Option<RequiredItemResolution<Self>> {
        compose_item_by(key, items, |item| item.value.clone(), conflicts)
    }
}

/// Required, forbidden, and closed inputs grouped by merge identity.
struct ItemGroups<Item>
where
    Item: FileItemRequirement,
{
    /// Required item assertions by item identity.
    required: ItemAssertionGroups<Item>,
    /// Forbidden item assertions by item identity.
    forbidden: ItemAssertionGroups<Item>,
    /// Policies that closed the collection.
    closed_by: Vec<Contributor>,
    /// Closed collection inputs with the allowed required identities.
    closed_inputs: Vec<ClosedInput<Item>>,
}

impl<Item> Default for ItemGroups<Item>
where
    Item: FileItemRequirement,
{
    fn default() -> Self {
        Self {
            required: ItemAssertionGroups::<Item>::new(),
            forbidden: ItemAssertionGroups::<Item>::new(),
            closed_by: Vec::new(),
            closed_inputs: Vec::new(),
        }
    }
}

/// Group raw item requirements before resolving each identity.
fn collect_item_groups<Item>(input: Vec<ItemInput<Item>>, grouped: &mut ItemGroups<Item>)
where
    Item: FileItemRequirement,
{
    for (prov, items) in input {
        let allowed = items
            .required
            .iter()
            .map(|(item, _)| item.merge_identity())
            .collect::<BTreeSet<_>>();
        for (item, msg) in items.required {
            grouped
                .required
                .entry(item.merge_identity())
                .or_default()
                .push((prov.clone(), (item, msg)));
        }
        for (item, msg) in items.forbidden {
            grouped
                .forbidden
                .entry(item.merge_identity())
                .or_default()
                .push((prov.clone(), (item, msg)));
        }
        if let Some(msg) = items.closed {
            grouped
                .closed_inputs
                .push((prov.clone(), msg.clone(), allowed));
            grouped.closed_by.push((prov, msg));
        }
    }
}

/// Resolve required item assertions for each item identity.
fn resolve_required_items<Item>(
    key: &str,
    required: ItemAssertionGroups<Item>,
    conflicts: &mut Vec<ConflictEntry>,
) -> ItemRequirementMap<Item>
where
    Item: FileItemRequirement,
    Item::Identity: ToString,
{
    let mut resolved_required = ItemRequirementMap::<Item>::new();
    for (identity, items) in required {
        let path = format!("{}.{}", key, identity.to_string());
        if let Some(resolved) = Item::compose_item(&path, items, conflicts) {
            let _ = resolved_required.insert(identity, resolved);
        }
    }
    resolved_required
}

/// Resolve forbidden item assertions for each item identity.
fn resolve_forbidden_items<Item>(forbidden: ItemAssertionGroups<Item>) -> ForbiddenItemMap<Item>
where
    Item: FileItemRequirement,
{
    let mut resolved_forbidden = ForbiddenItemMap::<Item>::new();
    for (identity, items) in forbidden {
        let Some((_, (first, _))) = items.first() else {
            continue;
        };
        let _ = resolved_forbidden.insert(
            identity,
            ResolvedRequirement {
                merged: first.clone(),
                collected: items
                    .into_iter()
                    .map(|(prov, (_, msg))| (prov, msg))
                    .collect(),
            },
        );
    }
    resolved_forbidden
}

/// Report identities that are both required and forbidden.
fn report_required_forbidden_conflicts<Item>(
    key: &str,
    resolved_required: &ItemRequirementMap<Item>,
    resolved_forbidden: &ForbiddenItemMap<Item>,
    conflicts: &mut Vec<ConflictEntry>,
) where
    Item: FileItemRequirement,
    Item::Identity: ToString,
{
    for identity in resolved_required.keys() {
        if let Some(forbidden) = resolved_forbidden.get(identity) {
            let mut contributors = Vec::new();
            if let Some(req) = resolved_required.get(identity) {
                contributors.extend(
                    req.collected
                        .iter()
                        .map(|(prov, _)| required_contributor(prov)),
                );
            }
            contributors.extend(
                forbidden
                    .collected
                    .iter()
                    .map(|(prov, _)| forbidden_contributor(prov)),
            );
            conflicts.push(ConflictEntry {
                key: format!("{}.{}", key, identity.to_string()),
                reason: "item-required-and-forbidden".to_owned(),
                contributors,
            });
        }
    }
}

/// Report required items rejected by a closed collection.
fn report_closed_collection_conflicts<Item>(
    key: &str,
    closed_inputs: &[ClosedInput<Item>],
    resolved_required: &ItemRequirementMap<Item>,
    conflicts: &mut Vec<ConflictEntry>,
) where
    Item: FileItemRequirement,
    Item::Identity: ToString,
{
    for (closer, _, allowed) in closed_inputs {
        for (identity, req) in resolved_required {
            if allowed.contains(identity) {
                continue;
            }
            let mut contributors = vec![(closer.clone(), "closed".to_owned())];
            contributors.extend(
                req.collected
                    .iter()
                    .map(|(prov, _)| required_contributor(prov)),
            );
            conflicts.push(ConflictEntry {
                key: format!("{}.{}", key, identity.to_string()),
                reason: "closed-collection-rejects-unlisted-required-item".to_owned(),
                contributors,
            });
        }
    }
}

/// Render a required-item contributor.
fn required_contributor(prov: &Provenance) -> Contributor {
    (prov.clone(), "required".to_owned())
}

/// Render a forbidden-item contributor.
fn forbidden_contributor(prov: &Provenance) -> Contributor {
    (prov.clone(), "forbidden".to_owned())
}
