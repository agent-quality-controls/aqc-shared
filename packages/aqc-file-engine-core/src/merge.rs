//! Shared merge machinery for file-engine requirements.
//!
//! Adapters emit plain engine requirements tagged with provenance. Engine merge
//! code composes those plain requirements into resolved values, while keeping
//! the collected assertions needed for precise findings.

use std::collections::{BTreeMap, BTreeSet};

use crate::toml_helpers::parse_version_tuple;
use crate::types::{ConfigScalar, Provenance};

/// One key on which policies irreconcilably disagree, with each value.
#[derive(Debug, Clone)]
pub struct ConflictEntry {
    /// The disagreeing key.
    pub key: String,
    /// Which composition rule found the conflict.
    pub reason: String,
    /// Each provenance paired with its value, rendered for display.
    pub contributors: Vec<(Provenance, String)>,
}

/// A composed requirement plus the policy assertions used to compose it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedRequirement<Merged, A> {
    pub merged: Merged,
    pub collected: Vec<(Provenance, A)>,
}

/// Product requirement for collections of identifiable file items.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemRequirements<Item> {
    pub required: Vec<(Item, String)>,
    pub banned: Vec<(Item, String)>,
    pub closed: Option<String>,
}

impl<Item> Default for ItemRequirements<Item> {
    fn default() -> Self {
        Self {
            required: Vec::new(),
            banned: Vec::new(),
            closed: None,
        }
    }
}

/// Resolved item requirements with attribution on every member.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedItemRequirements<Item>
where
    Item: FileItemRequirement,
{
    pub required: BTreeMap<Item::Identity, ResolvedRequirement<Item, (Item, String)>>,
    pub banned: BTreeMap<Item::Identity, ResolvedRequirement<Item, String>>,
    pub closed_by: Vec<(Provenance, String)>,
}

impl<Item> Default for ResolvedItemRequirements<Item>
where
    Item: FileItemRequirement,
{
    fn default() -> Self {
        Self {
            required: BTreeMap::new(),
            banned: BTreeMap::new(),
            closed_by: Vec::new(),
        }
    }
}

/// A file-item requirement that can identify and compose matching policy input.
pub trait FileItemRequirement: Sized + Clone {
    type Identity: Ord + Clone;

    fn merge_identity(&self) -> Self::Identity;

    fn compose_item(
        key: &str,
        items: Vec<(Provenance, (Self, String))>,
        conflicts: &mut Vec<ConflictEntry>,
    ) -> Option<ResolvedRequirement<Self, (Self, String)>>;
}

/// Ban-only requirement for collections where policy names a pattern.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PatternBanRequirements<Pattern> {
    pub banned: Vec<(Pattern, String)>,
}

impl<Pattern> Default for PatternBanRequirements<Pattern> {
    fn default() -> Self {
        Self { banned: Vec::new() }
    }
}

/// Resolved pattern bans with attribution on each pattern.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedPatternBanRequirements<Pattern>
where
    Pattern: PatternBanRequirement,
{
    pub banned: BTreeMap<Pattern::Identity, ResolvedRequirement<Pattern, String>>,
}

impl<Pattern> Default for ResolvedPatternBanRequirements<Pattern>
where
    Pattern: PatternBanRequirement,
{
    fn default() -> Self {
        Self {
            banned: BTreeMap::new(),
        }
    }
}

/// A ban pattern that can be deduped across policy input.
pub trait PatternBanRequirement: Sized + Clone {
    type Identity: Ord + Clone;

    fn merge_identity(&self) -> Self::Identity;

    fn render(&self) -> String;
}

/// Requirement for collections where the file key is the item identity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyedItem<Value> {
    pub file_key: String,
    pub value: Value,
}

/// Product requirement for list-like TOML fields.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ListRequirements {
    pub contains: BTreeMap<String, String>,
    pub excludes: BTreeMap<String, String>,
    pub exact: Option<(Vec<String>, String)>,
}

/// Resolved list requirements with per-item and exact-list attribution.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ResolvedListRequirements {
    pub contains: BTreeMap<String, ResolvedRequirement<(), String>>,
    pub excludes: BTreeMap<String, ResolvedRequirement<(), String>>,
    pub exact: Option<ResolvedRequirement<Vec<String>, (Vec<String>, String)>>,
}

/// Assertion types that compose several policy assertions into one value.
pub trait Resolve: Sized + Clone {
    type Merged: Clone;

    fn resolve(
        key: &str,
        items: Vec<(Provenance, Self)>,
        conflicts: &mut Vec<ConflictEntry>,
    ) -> Option<ResolvedRequirement<Self::Merged, Self>>;
}

/// Compose assertions keyed by field name.
pub fn resolve_map<K, A>(
    input: Vec<(Provenance, BTreeMap<K, A>)>,
    key_path: impl Fn(&K) -> String,
    conflicts: &mut Vec<ConflictEntry>,
) -> BTreeMap<K, ResolvedRequirement<A::Merged, A>>
where
    K: Ord + Clone,
    A: Resolve,
{
    let mut by_key: BTreeMap<K, Vec<(Provenance, A)>> = BTreeMap::new();
    for (prov, map) in input {
        for (key, assertion) in map {
            by_key
                .entry(key)
                .or_default()
                .push((prov.clone(), assertion));
        }
    }

    let mut out = BTreeMap::new();
    for (key, items) in by_key {
        if let Some(resolved) = A::resolve(&key_path(&key), items, conflicts) {
            let _ = out.insert(key, resolved);
        }
    }
    out
}

/// Compose an optional singleton assertion.
pub fn resolve_maybe<A>(
    key: &str,
    input: Vec<(Provenance, Option<A>)>,
    conflicts: &mut Vec<ConflictEntry>,
) -> Option<ResolvedRequirement<A::Merged, A>>
where
    A: Resolve,
{
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

/// Compose file-item collection requirements.
pub fn resolve_items<Item>(
    key: &str,
    input: Vec<(Provenance, ItemRequirements<Item>)>,
    conflicts: &mut Vec<ConflictEntry>,
) -> ResolvedItemRequirements<Item>
where
    Item: FileItemRequirement,
    Item::Identity: ToString,
{
    let mut required: BTreeMap<Item::Identity, Vec<(Provenance, (Item, String))>> = BTreeMap::new();
    let mut banned: BTreeMap<Item::Identity, Vec<(Provenance, (Item, String))>> = BTreeMap::new();
    let mut closed_by = Vec::new();
    let mut closed_inputs: Vec<(Provenance, String, BTreeSet<Item::Identity>)> = Vec::new();

    for (prov, items) in input {
        let allowed = items
            .required
            .iter()
            .map(|(item, _)| item.merge_identity())
            .collect::<BTreeSet<_>>();
        for (item, msg) in items.required {
            required
                .entry(item.merge_identity())
                .or_default()
                .push((prov.clone(), (item, msg)));
        }
        for (item, msg) in items.banned {
            banned
                .entry(item.merge_identity())
                .or_default()
                .push((prov.clone(), (item, msg)));
        }
        if let Some(msg) = items.closed {
            closed_inputs.push((prov.clone(), msg.clone(), allowed));
            closed_by.push((prov, msg));
        }
    }

    let mut resolved_required = BTreeMap::new();
    for (identity, items) in required {
        let path = format!("{}.{}", key, identity.to_string());
        if let Some(resolved) = Item::compose_item(&path, items, conflicts) {
            let _ = resolved_required.insert(identity, resolved);
        }
    }

    let mut resolved_banned = BTreeMap::new();
    for (identity, items) in banned {
        let Some((_, (first, _))) = items.first() else {
            continue;
        };
        let _ = resolved_banned.insert(
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

    for identity in resolved_required.keys() {
        if let Some(ban) = resolved_banned.get(identity) {
            let mut contributors = Vec::new();
            if let Some(req) = resolved_required.get(identity) {
                contributors.extend(req.collected.iter().map(|(prov, entry)| {
                    let _ = entry;
                    (prov.clone(), "required".to_owned())
                }));
            }
            contributors.extend(
                ban.collected
                    .iter()
                    .map(|(prov, _)| (prov.clone(), "banned".to_owned())),
            );
            conflicts.push(ConflictEntry {
                key: format!("{}.{}", key, identity.to_string()),
                reason: "item-required-and-banned".to_owned(),
                contributors,
            });
        }
    }

    for (closer, _, allowed) in &closed_inputs {
        for (identity, req) in &resolved_required {
            if allowed.contains(identity) {
                continue;
            }
            let mut contributors = vec![(closer.clone(), "closed".to_owned())];
            contributors.extend(
                req.collected
                    .iter()
                    .map(|(prov, _)| (prov.clone(), "required".to_owned())),
            );
            conflicts.push(ConflictEntry {
                key: format!("{}.{}", key, identity.to_string()),
                reason: "closed-collection-rejects-unlisted-required-item".to_owned(),
                contributors,
            });
        }
    }

    ResolvedItemRequirements {
        required: resolved_required,
        banned: resolved_banned,
        closed_by,
    }
}

/// Compose ban-only pattern requirements.
pub fn resolve_pattern_bans<Pattern>(
    key: &str,
    input: Vec<(Provenance, PatternBanRequirements<Pattern>)>,
    conflicts: &mut Vec<ConflictEntry>,
) -> ResolvedPatternBanRequirements<Pattern>
where
    Pattern: PatternBanRequirement,
    Pattern::Identity: ToString,
{
    let _ = conflicts;
    let mut banned: BTreeMap<Pattern::Identity, Vec<(Provenance, (Pattern, String))>> =
        BTreeMap::new();

    for (prov, patterns) in input {
        for (pattern, msg) in patterns.banned {
            banned
                .entry(pattern.merge_identity())
                .or_default()
                .push((prov.clone(), (pattern, msg)));
        }
    }

    let mut resolved_banned = BTreeMap::new();
    for (identity, items) in banned {
        let Some((_, (first, _))) = items.first() else {
            continue;
        };
        let _path = format!("{}.{}", key, identity.to_string());
        let _ = resolved_banned.insert(
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

    ResolvedPatternBanRequirements {
        banned: resolved_banned,
    }
}

/// Compose list product requirements.
pub fn resolve_list(
    key: &str,
    items: Vec<(Provenance, ListRequirements)>,
    conflicts: &mut Vec<ConflictEntry>,
) -> ResolvedListRequirements {
    let mut contains: BTreeMap<String, Vec<(Provenance, String)>> = BTreeMap::new();
    let mut excludes: BTreeMap<String, Vec<(Provenance, String)>> = BTreeMap::new();
    let mut exact_items = Vec::new();

    for (prov, list) in items {
        for (name, msg) in list.contains {
            contains.entry(name).or_default().push((prov.clone(), msg));
        }
        for (name, msg) in list.excludes {
            excludes.entry(name).or_default().push((prov.clone(), msg));
        }
        if let Some(exact) = list.exact {
            exact_items.push((prov, exact));
        }
    }

    let mut resolved_contains = BTreeMap::new();
    for (name, collected) in contains {
        let _ = resolved_contains.insert(
            name,
            ResolvedRequirement {
                merged: (),
                collected,
            },
        );
    }

    let mut resolved_excludes = BTreeMap::new();
    for (name, collected) in excludes {
        let _ = resolved_excludes.insert(
            name,
            ResolvedRequirement {
                merged: (),
                collected,
            },
        );
    }

    for name in resolved_contains.keys() {
        if let Some(exclude) = resolved_excludes.get(name) {
            let mut contributors = Vec::new();
            if let Some(include) = resolved_contains.get(name) {
                contributors.extend(
                    include
                        .collected
                        .iter()
                        .map(|(prov, _)| (prov.clone(), "contains".to_owned())),
                );
            }
            contributors.extend(
                exclude
                    .collected
                    .iter()
                    .map(|(prov, _)| (prov.clone(), "excludes".to_owned())),
            );
            conflicts.push(ConflictEntry {
                key: format!("{key}.{name}"),
                reason: "list-contains-and-excludes".to_owned(),
                contributors,
            });
        }
    }

    let exact = resolve_exact_list(key, exact_items, conflicts);
    if let Some(exact) = &exact {
        let allowed = exact.merged.iter().cloned().collect::<BTreeSet<_>>();
        for (name, include) in &resolved_contains {
            if !allowed.contains(name) {
                let mut contributors = exact
                    .collected
                    .iter()
                    .map(|(prov, _)| (prov.clone(), "exact".to_owned()))
                    .collect::<Vec<_>>();
                contributors.extend(
                    include
                        .collected
                        .iter()
                        .map(|(prov, _)| (prov.clone(), "contains".to_owned())),
                );
                conflicts.push(ConflictEntry {
                    key: format!("{key}.{name}"),
                    reason: "list-exact-missing-contained-item".to_owned(),
                    contributors,
                });
            }
        }
        for (name, exclude) in &resolved_excludes {
            if allowed.contains(name) {
                let mut contributors = exact
                    .collected
                    .iter()
                    .map(|(prov, _)| (prov.clone(), "exact".to_owned()))
                    .collect::<Vec<_>>();
                contributors.extend(
                    exclude
                        .collected
                        .iter()
                        .map(|(prov, _)| (prov.clone(), "excludes".to_owned())),
                );
                conflicts.push(ConflictEntry {
                    key: format!("{key}.{name}"),
                    reason: "list-exact-contains-excluded-item".to_owned(),
                    contributors,
                });
            }
        }
    }

    ResolvedListRequirements {
        contains: resolved_contains,
        excludes: resolved_excludes,
        exact,
    }
}

/// Compose assertions that must all be the same semantic value.
pub fn resolve_scalar<T>(
    key: &str,
    items: Vec<(Provenance, T)>,
    render: impl Fn(&T) -> String,
    conflicts: &mut Vec<ConflictEntry>,
) -> Option<ResolvedRequirement<T, T>>
where
    T: PartialEq + Clone,
{
    resolve_all_equal(key, "scalar-disagree", items, render, conflicts)
}

/// Compose exact-list assertions.
pub fn resolve_exact_list(
    key: &str,
    items: Vec<(Provenance, (Vec<String>, String))>,
    conflicts: &mut Vec<ConflictEntry>,
) -> Option<ResolvedRequirement<Vec<String>, (Vec<String>, String)>> {
    let resolved = resolve_all_equal(
        key,
        "exact-mismatch",
        items,
        |(list, _)| format!("{list:?}"),
        conflicts,
    )?;
    Some(ResolvedRequirement {
        merged: resolved.merged.0,
        collected: resolved.collected,
    })
}

/// Compose semantic values and retain every assertion.
pub fn resolve_all_equal<T>(
    key: &str,
    reason: &str,
    items: Vec<(Provenance, T)>,
    render: impl Fn(&T) -> String,
    conflicts: &mut Vec<ConflictEntry>,
) -> Option<ResolvedRequirement<T, T>>
where
    T: PartialEq + Clone,
{
    let mut iter = items.iter();
    let (_, first) = iter.next()?;
    let disagree = iter.any(|(_, value)| value != first);
    if disagree {
        conflicts.push(ConflictEntry {
            key: key.to_owned(),
            reason: reason.to_owned(),
            contributors: items
                .iter()
                .map(|(prov, value)| (prov.clone(), render(value)))
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

/// Compose two optional scalar fields inside a larger entry.
pub fn compose_optional_field<T>(
    key: &str,
    items: Vec<(Provenance, Option<T>)>,
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

/// Ordered, deduplicated union of string lists.
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

/// Union of string sets.
#[must_use]
pub fn compose_string_set(items: Vec<BTreeSet<String>>) -> BTreeSet<String> {
    items.into_iter().flatten().collect()
}

/// Highest version-like floor, retaining the winning message.
#[must_use]
pub fn strongest_version_floor(items: Vec<(String, String)>) -> (String, String) {
    items
        .into_iter()
        .max_by(|(a, _), (b, _)| parse_version_tuple(a).cmp(&parse_version_tuple(b)))
        .unwrap_or_default()
}

/// Semantic equality of two keyed `(value, message)` maps.
#[must_use]
pub fn keyed_entries_eq<S: PartialEq, M>(
    a: &BTreeMap<String, (S, M)>,
    b: &BTreeMap<String, (S, M)>,
) -> bool {
    a.len() == b.len()
        && a.iter()
            .all(|(key, (left, _))| b.get(key).is_some_and(|(right, _)| left == right))
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
        items: Vec<(Provenance, (Self, String))>,
        conflicts: &mut Vec<ConflictEntry>,
    ) -> Option<ResolvedRequirement<Self, (Self, String)>> {
        compose_item_by(key, items, |item| item.value.clone(), conflicts)
    }
}

impl Resolve for ConfigScalar {
    type Merged = Self;

    fn resolve(
        key: &str,
        items: Vec<(Provenance, Self)>,
        conflicts: &mut Vec<ConflictEntry>,
    ) -> Option<ResolvedRequirement<Self::Merged, Self>> {
        resolve_scalar(key, items, |item| format!("{item:?}"), conflicts)
    }
}

/// Generic item composer for semantic-value equality.
pub fn compose_item_by<Item, Semantic>(
    key: &str,
    items: Vec<(Provenance, (Item, String))>,
    project: impl Fn(&Item) -> Semantic,
    conflicts: &mut Vec<ConflictEntry>,
) -> Option<ResolvedRequirement<Item, (Item, String)>>
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
                .map(|(prov, _)| (prov.clone(), "required".to_owned()))
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
