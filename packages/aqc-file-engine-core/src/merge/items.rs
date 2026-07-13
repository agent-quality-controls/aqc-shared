//! Item and forbidden-glob merge functions.

use std::collections::BTreeSet;

use super::{
    ConflictEntry, Contributor, ExactItemsInput, FileItemRequirement, ForbiddenGlobRequirement,
    ForbiddenItemMap, GlobAssertionGroups, GlobInput, GlobResolutionMap, ItemAssertionGroups,
    ItemAssertionInput, ItemInput, ItemRequirementMap, KeyedItem, RequiredItemResolution,
    ResolvedExactItems, ResolvedForbiddenGlobRequirements, ResolvedItemRequirements,
    ResolvedRequirement, sort_provenanced,
};
use crate::types::Provenance;

/// Every positively asserted item, whether supplied through `required` or
/// through a complete `exact` collection, with each identity returned once.
pub fn asserted_items<Item>(
    resolved: &ResolvedItemRequirements<Item>,
) -> impl Iterator<Item = (&Item::Identity, &RequiredItemResolution<Item>)>
where
    Item: FileItemRequirement,
{
    let exact = resolved.exact.as_ref();
    resolved
        .required
        .iter()
        .filter(move |(identity, _)| {
            exact.is_none_or(|complete| !complete.identities.contains(*identity))
        })
        .chain(exact.into_iter().flat_map(|complete| complete.items.iter()))
}

pub fn resolve_items<Item>(
    key: &str,
    mut input: Vec<ItemInput<Item>>,
    conflicts: &mut Vec<ConflictEntry>,
) -> ResolvedItemRequirements<Item>
where
    Item: FileItemRequirement,
    Item::Identity: ToString,
{
    sort_provenanced(&mut input);
    let mut grouped = ItemGroups::default();
    collect_item_groups(input, &mut grouped);

    let resolved_required = resolve_required_items(key, grouped.required.clone(), conflicts);
    let resolved_forbidden = resolve_forbidden_items(grouped.forbidden);

    report_required_forbidden_conflicts(key, &resolved_required, &resolved_forbidden, conflicts);
    report_exact_conflicts(
        key,
        &grouped.exact_inputs,
        &resolved_required,
        &resolved_forbidden,
        conflicts,
    );

    let exact = resolve_exact_items(
        key,
        grouped.required,
        grouped.exact_items,
        &grouped.exact_inputs,
        conflicts,
    );

    ResolvedItemRequirements {
        required: resolved_required,
        forbidden: resolved_forbidden,
        exact,
    }
}

pub fn resolve_forbidden_globs<Glob>(
    _key: &str,
    mut input: Vec<GlobInput<Glob>>,
    conflicts: &mut Vec<ConflictEntry>,
) -> ResolvedForbiddenGlobRequirements<Glob>
where
    Glob: ForbiddenGlobRequirement,
    Glob::Identity: ToString,
{
    conflicts.reserve(0);
    sort_provenanced(&mut input);
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
    mut items: Vec<ItemAssertionInput<Item>>,
    project: impl Fn(&Item) -> Semantic,
    conflicts: &mut Vec<ConflictEntry>,
) -> Option<RequiredItemResolution<Item>>
where
    Item: Clone,
    Semantic: PartialEq,
{
    sort_provenanced(&mut items);
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
                .map(|(prov, (_, message))| (prov.clone(), message.clone()))
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

/// Required, forbidden, and exact inputs grouped by merge identity.
struct ItemGroups<Item>
where
    Item: FileItemRequirement,
{
    /// Required item assertions by item identity.
    required: ItemAssertionGroups<Item>,
    /// Forbidden item assertions by item identity.
    forbidden: ItemAssertionGroups<Item>,
    /// Exact item assertions by item identity.
    exact_items: ItemAssertionGroups<Item>,
    /// Explicit complete collections supplied by policies.
    exact_inputs: Vec<ExactItemsInput<Item>>,
}

impl<Item> Default for ItemGroups<Item>
where
    Item: FileItemRequirement,
{
    fn default() -> Self {
        Self {
            required: ItemAssertionGroups::<Item>::new(),
            forbidden: ItemAssertionGroups::<Item>::new(),
            exact_items: ItemAssertionGroups::<Item>::new(),
            exact_inputs: Vec::new(),
        }
    }
}

/// Group raw item requirements before resolving each identity.
fn collect_item_groups<Item>(input: Vec<ItemInput<Item>>, grouped: &mut ItemGroups<Item>)
where
    Item: FileItemRequirement,
{
    for (prov, items) in input {
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
        if let Some((exact, msg)) = items.exact {
            for item in &exact {
                grouped
                    .exact_items
                    .entry(item.merge_identity())
                    .or_default()
                    .push((prov.clone(), (item.clone(), msg.clone())));
            }
            grouped.exact_inputs.push((prov, (exact, msg)));
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

/// Report identity disagreements and exact conflicts with required/forbidden assertions.
fn report_exact_conflicts<Item>(
    key: &str,
    exact_inputs: &[ExactItemsInput<Item>],
    resolved_required: &ItemRequirementMap<Item>,
    resolved_forbidden: &ForbiddenItemMap<Item>,
    conflicts: &mut Vec<ConflictEntry>,
) where
    Item: FileItemRequirement,
    Item::Identity: ToString,
{
    let identity_sets = exact_inputs
        .iter()
        .map(|(_, (items, _))| {
            items
                .iter()
                .map(FileItemRequirement::merge_identity)
                .collect::<BTreeSet<_>>()
        })
        .collect::<Vec<_>>();

    if identity_sets
        .windows(2)
        .any(|sets| sets.first() != sets.get(1))
    {
        conflicts.push(ConflictEntry {
            key: key.to_owned(),
            reason: "exact-item-identities-disagree".to_owned(),
            contributors: exact_inputs
                .iter()
                .map(|(prov, (_, msg))| (prov.clone(), msg.clone()))
                .collect(),
        });
    }

    for ((exact_prov, (_, exact_msg)), allowed) in exact_inputs.iter().zip(&identity_sets) {
        for (identity, req) in resolved_required {
            if allowed.contains(identity) {
                continue;
            }
            let mut contributors = vec![(exact_prov.clone(), exact_msg.clone())];
            contributors.extend(
                req.collected
                    .iter()
                    .map(|(prov, _)| required_contributor(prov)),
            );
            conflicts.push(ConflictEntry {
                key: format!("{}.{}", key, identity.to_string()),
                reason: "exact-items-reject-unlisted-required-item".to_owned(),
                contributors,
            });
        }

        for identity in allowed {
            if let Some(forbidden) = resolved_forbidden.get(identity) {
                let mut contributors = vec![(exact_prov.clone(), exact_msg.clone())];
                contributors.extend(
                    forbidden
                        .collected
                        .iter()
                        .map(|(prov, _)| forbidden_contributor(prov)),
                );
                conflicts.push(ConflictEntry {
                    key: format!("{}.{}", key, identity.to_string()),
                    reason: "exact-item-is-forbidden".to_owned(),
                    contributors,
                });
            }
        }
    }
}

/// Build resolved exact state from agreeing identities and composed values.
fn resolve_exact_items<Item>(
    key: &str,
    mut required: ItemAssertionGroups<Item>,
    exact_items: ItemAssertionGroups<Item>,
    exact_inputs: &[ExactItemsInput<Item>],
    conflicts: &mut Vec<ConflictEntry>,
) -> Option<ResolvedExactItems<Item>>
where
    Item: FileItemRequirement,
    Item::Identity: ToString,
{
    let (_, (first, _)) = exact_inputs.first()?;
    let identities = first
        .iter()
        .map(FileItemRequirement::merge_identity)
        .collect::<BTreeSet<_>>();
    for (identity, assertions) in exact_items {
        required.entry(identity).or_default().extend(assertions);
    }
    let items = resolve_required_items(key, required, conflicts)
        .into_iter()
        .filter(|(identity, _)| identities.contains(identity))
        .collect();
    Some(ResolvedExactItems {
        identities,
        items,
        collected: exact_inputs.to_vec(),
    })
}

/// Render a required-item contributor.
fn required_contributor(prov: &Provenance) -> Contributor {
    (prov.clone(), "required".to_owned())
}

/// Render a forbidden-item contributor.
fn forbidden_contributor(prov: &Provenance) -> Contributor {
    (prov.clone(), "forbidden".to_owned())
}
