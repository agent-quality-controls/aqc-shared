//! Shared requirement merge helpers.

#![cfg_attr(
    not(test),
    expect(
        clippy::missing_docs_in_private_items,
        reason = "Private aggregate resolution helpers are internal requirement steps."
    )
)]
#![allow(
    clippy::type_complexity,
    reason = "Resolution helpers preserve the public requirement map shapes."
)]

use std::collections::BTreeMap;

use aqc_file_engine_core::{
    ConflictEntry, FileItemRequirement, ForbiddenGlobRequirement, ForbiddenGlobRequirements,
    ItemRequirements, Provenance, ResolvedForbiddenGlobRequirements, ResolvedItemRequirements,
    resolve_forbidden_globs, resolve_items,
};

use super::super::profiles::{ProfileRequirements, ResolvedProfileRequirements};

pub(super) fn resolve_forbidden_glob_map<K, Glob>(
    input: Vec<(Provenance, BTreeMap<K, ForbiddenGlobRequirements<Glob>>)>,
    key_path: impl Fn(&K) -> String,
    conflicts: &mut Vec<ConflictEntry>,
) -> BTreeMap<K, ResolvedForbiddenGlobRequirements<Glob>>
where
    K: Ord + Clone,
    Glob: ForbiddenGlobRequirement,
    Glob::Identity: ToString,
{
    let mut by_key: BTreeMap<K, Vec<(Provenance, ForbiddenGlobRequirements<Glob>)>> =
        BTreeMap::new();
    for (prov, map) in input {
        for (key, globs) in map {
            by_key.entry(key).or_default().push((prov.clone(), globs));
        }
    }
    by_key
        .into_iter()
        .map(|(key, globs)| {
            let path = key_path(&key);
            (key, resolve_forbidden_globs(&path, globs, conflicts))
        })
        .collect()
}

pub(super) fn resolve_maybe_forbidden_globs<Glob>(
    key: &str,
    input: Vec<(Provenance, Option<ForbiddenGlobRequirements<Glob>>)>,
    conflicts: &mut Vec<ConflictEntry>,
) -> Option<ResolvedForbiddenGlobRequirements<Glob>>
where
    Glob: ForbiddenGlobRequirement,
    Glob::Identity: ToString,
{
    let globs = input
        .into_iter()
        .filter_map(|(prov, globs)| globs.map(|inner| (prov, inner)))
        .collect::<Vec<_>>();
    if globs.is_empty() {
        None
    } else {
        Some(resolve_forbidden_globs(key, globs, conflicts))
    }
}

pub(super) fn resolve_item_map<K, Item>(
    input: Vec<(Provenance, BTreeMap<K, ItemRequirements<Item>>)>,
    key_path: impl Fn(&K) -> String,
    conflicts: &mut Vec<ConflictEntry>,
) -> BTreeMap<K, ResolvedItemRequirements<Item>>
where
    K: Ord + Clone,
    Item: FileItemRequirement,
    Item::Identity: ToString,
{
    let mut by_key: BTreeMap<K, Vec<(Provenance, ItemRequirements<Item>)>> = BTreeMap::new();
    for (prov, map) in input {
        for (key, items) in map {
            by_key.entry(key).or_default().push((prov.clone(), items));
        }
    }
    by_key
        .into_iter()
        .map(|(key, items)| {
            let path = key_path(&key);
            (key, resolve_items(&path, items, conflicts))
        })
        .collect()
}

pub(super) fn resolve_maybe_items<Item>(
    key: &str,
    input: Vec<(Provenance, Option<ItemRequirements<Item>>)>,
    conflicts: &mut Vec<ConflictEntry>,
) -> Option<ResolvedItemRequirements<Item>>
where
    Item: FileItemRequirement,
    Item::Identity: ToString,
{
    let items = input
        .into_iter()
        .filter_map(|(prov, item_requirements)| item_requirements.map(|inner| (prov, inner)))
        .collect::<Vec<_>>();
    if items.is_empty() {
        None
    } else {
        Some(resolve_items(key, items, conflicts))
    }
}

pub(super) fn resolve_profile_map(
    input: Vec<(Provenance, BTreeMap<String, ProfileRequirements>)>,
    conflicts: &mut Vec<ConflictEntry>,
) -> BTreeMap<String, ResolvedProfileRequirements> {
    let mut by_key: BTreeMap<String, Vec<(Provenance, ProfileRequirements)>> = BTreeMap::new();
    for (prov, map) in input {
        for (profile, req) in map {
            by_key.entry(profile).or_default().push((prov.clone(), req));
        }
    }
    by_key
        .into_iter()
        .map(|(profile, items)| {
            let key = format!("[profile.{profile}]");
            (
                profile,
                ProfileRequirements::resolve(&key, items, conflicts),
            )
        })
        .collect()
}
