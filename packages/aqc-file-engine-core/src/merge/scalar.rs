//! Scalar, optional, and map merge functions.

use std::collections::BTreeSet;

use super::{
    ConflictEntry, GroupedAssertions, KeyedValueMap, MapInputs, OptionalInput, Provenanced,
    Resolve, ResolvedAssertionOption, ResolvedMap, ResolvedRequirement, ResolvedSameOption,
    ScalarValue, VersionFloor, sort_provenanced,
};
use crate::types::ConfigScalar;
use crate::version::parse_version_tuple;

pub fn resolve_map<K, A>(
    mut input: MapInputs<K, A>,
    key_path: impl Fn(&K) -> String,
    conflicts: &mut Vec<ConflictEntry>,
) -> ResolvedMap<K, A>
where
    K: Ord + Clone,
    A: Resolve,
{
    sort_provenanced(&mut input);
    let mut by_key = GroupedAssertions::<K, A>::new();
    for (prov, map) in input {
        for (key, assertion) in map {
            by_key
                .entry(key)
                .or_default()
                .push((prov.clone(), assertion));
        }
    }

    let mut out = std::collections::BTreeMap::new();
    for (key, items) in by_key {
        if let Some(resolved) = A::resolve(&key_path(&key), items, conflicts) {
            let _ = out.insert(key, resolved);
        }
    }
    out
}

pub fn resolve_maybe<A>(
    key: &str,
    mut input: Vec<OptionalInput<A>>,
    conflicts: &mut Vec<ConflictEntry>,
) -> ResolvedAssertionOption<A>
where
    A: Resolve,
{
    sort_provenanced(&mut input);
    let items = input
        .into_iter()
        .filter_map(|(prov, value)| value.map(|assertion| (prov, assertion)))
        .collect::<Vec<_>>();
    if items.is_empty() {
        None
    } else {
        A::resolve(key, items, conflicts)
    }
}

pub fn resolve_scalar<T>(
    key: &str,
    items: Vec<Provenanced<T>>,
    render: impl Fn(&T) -> String,
    conflicts: &mut Vec<ConflictEntry>,
) -> ResolvedSameOption<T>
where
    T: PartialEq + Clone,
{
    resolve_all_equal(key, "scalar-disagree", items, render, conflicts)
}

pub fn resolve_all_equal<T>(
    key: &str,
    reason: &str,
    mut items: Vec<Provenanced<T>>,
    render: impl Fn(&T) -> String,
    conflicts: &mut Vec<ConflictEntry>,
) -> ResolvedSameOption<T>
where
    T: PartialEq + Clone,
{
    sort_provenanced(&mut items);
    let mut iter = items.iter();
    let (_, first) = iter.next()?;
    let disagree = iter.any(|(_, value)| value != first);
    if disagree {
        push_conflict(key, reason, &items, render, conflicts);
        None
    } else {
        Some(ResolvedRequirement {
            merged: first.clone(),
            collected: items,
        })
    }
}

pub fn push_conflict<T>(
    key: impl Into<String>,
    reason: impl Into<String>,
    items: &[Provenanced<T>],
    render: impl Fn(&T) -> String,
    conflicts: &mut Vec<ConflictEntry>,
) {
    if items.is_empty() {
        return;
    }
    conflicts.push(ConflictEntry {
        key: key.into(),
        reason: reason.into(),
        contributors: items
            .iter()
            .map(|(prov, value)| (prov.clone(), render(value)))
            .collect(),
    });
}

pub fn compose_optional_field<T>(
    key: &str,
    items: Vec<OptionalInput<T>>,
    render: impl Fn(&T) -> String,
    conflicts: &mut Vec<ConflictEntry>,
) -> Option<T>
where
    T: PartialEq + Clone,
{
    let present = items
        .into_iter()
        .filter_map(|(prov, value)| value.map(|inner| (prov, inner)))
        .collect::<Vec<_>>();
    if present.is_empty() {
        None
    } else {
        resolve_scalar(key, present, render, conflicts).map(|resolved| resolved.merged)
    }
}

#[must_use]
pub fn compose_string_list(items: Vec<Vec<String>>) -> Vec<String> {
    let mut out = Vec::new();
    for list in items {
        for item in list {
            if !out.iter().any(|seen| seen == &item) {
                out.push(item);
            }
        }
    }
    out
}

#[must_use]
pub fn compose_string_set(items: Vec<BTreeSet<String>>) -> BTreeSet<String> {
    items.into_iter().flatten().collect()
}

#[must_use]
pub fn strongest_version_floor(items: Vec<VersionFloor>) -> VersionFloor {
    items
        .into_iter()
        .max_by(|(a, _), (b, _)| parse_version_tuple(a).cmp(&parse_version_tuple(b)))
        .unwrap_or_default()
}

#[must_use]
pub fn keyed_entries_eq<S: PartialEq, M>(a: &KeyedValueMap<S, M>, b: &KeyedValueMap<S, M>) -> bool {
    a.len() == b.len()
        && a.iter()
            .all(|(key, (left, _))| b.get(key).is_some_and(|(right, _)| left == right))
}

impl Resolve for ConfigScalar {
    type Merged = Self;

    fn resolve(
        key: &str,
        items: Vec<Provenanced<Self>>,
        conflicts: &mut Vec<ConflictEntry>,
    ) -> ResolvedAssertionOption<Self> {
        resolve_scalar(key, items, ScalarValue::render, conflicts)
    }
}
