//! The shared merge machinery: how contributions for one file converge.
//!
//! When many policies (via many adapters) write the same file, their
//! contributions all reach one engine as `MergedAssertion`s. This module is the
//! single place that knows *how* to combine them: union the contribution lists,
//! then resolve each key to one value or a conflict. It names no file type; each
//! engine's assertion types implement [`Resolve`], and nothing else (the broker,
//! the adapters) touches this code.

use std::collections::{BTreeMap, BTreeSet};

use crate::types::{MergedAssertion, Msg, Provenance};

/// A key's contributions: each source's provenance paired with its value.
pub type Contributions<A> = Vec<(Provenance, A)>;

/// One field across its keys: each key to its merged contributions
/// (e.g. `workspace_lints` keyed by tool name).
pub type AssertionMap<K, A> = BTreeMap<K, MergedAssertion<A>>;

/// One key on which sources irreconcilably disagree, with each source's value.
///
/// `key` is relative to the field; the engine prepends the field path to form
/// the full in-file key on the resulting `Finding::PolicyConflict`.
#[derive(Debug, Clone)]
pub struct ConflictEntry {
    /// The disagreeing key.
    pub key: String,
    /// Which resolution rule fired: `scalar-disagree`, `set-key-disagree`,
    /// or `exact-mismatch`.
    pub reason: String,
    /// Each contributing provenance paired with its value, rendered for display.
    pub contributors: Contributions<String>,
}

/// An assertion type that knows how to resolve multiple contributions for one
/// key into a single value, pushing a [`ConflictEntry`] for genuine disagreement.
///
/// This is the single decoupling seam: every file engine's assertion types
/// implement it; the generic strategies below do the work; the broker and the
/// adapters never call it.
pub trait Resolve: Sized + Clone {
    /// Resolve all contributions for one key (under `key`, the field's in-file
    /// path). Returns the merged value, or `None` and pushes one or more
    /// `ConflictEntry`s when the contributions cannot be reconciled.
    fn resolve(
        key: &str,
        contributions: Contributions<Self>,
        conflicts: &mut Vec<ConflictEntry>,
    ) -> Option<Self>;
}

/// Resolve a set of contributions that must all hold the same value.
///
/// Different values → one conflict keyed by `key`, tagged with `reason`, and
/// `None`. The two public entry points differ only in the reason they record.
fn resolve_all_equal<T>(
    key: &str,
    reason: &str,
    contributions: Contributions<T>,
    render: impl Fn(&T) -> String,
    conflicts: &mut Vec<ConflictEntry>,
) -> Option<T>
where
    T: PartialEq,
{
    let mut iter = contributions.into_iter();
    let (first_prov, first) = iter.next()?;
    let mut contributors: Contributions<String> = vec![(first_prov, render(&first))];
    let mut disagree = false;
    for (prov, value) in iter {
        if value != first {
            disagree = true;
        }
        contributors.push((prov, render(&value)));
    }
    if disagree {
        conflicts.push(ConflictEntry {
            key: key.to_owned(),
            reason: reason.to_owned(),
            contributors,
        });
        None
    } else {
        Some(first)
    }
}

/// Resolve a scalar: every contribution must hold the same value.
///
/// Different values → one conflict keyed by `key` (`scalar-disagree`), and `None`.
pub fn resolve_scalar<T>(
    key: &str,
    contributions: Contributions<T>,
    render: impl Fn(&T) -> String,
    conflicts: &mut Vec<ConflictEntry>,
) -> Option<T>
where
    T: PartialEq,
{
    resolve_all_equal(key, "scalar-disagree", contributions, render, conflicts)
}

/// Resolve an "exactly this" assertion: every contribution must be identical.
///
/// Same value-level check as [`resolve_scalar`]; the resolution for
/// `IsExactly`-style assertions (no key-wise union), tagged `exact-mismatch`.
pub fn resolve_exact<T>(
    key: &str,
    contributions: Contributions<T>,
    render: impl Fn(&T) -> String,
    conflicts: &mut Vec<ConflictEntry>,
) -> Option<T>
where
    T: PartialEq,
{
    resolve_all_equal(key, "exact-mismatch", contributions, render, conflicts)
}

/// Merge a set of `key -> value` maps.
///
/// The keys union; a key present in more than one map with different values →
/// one conflict keyed by `key_prefix.<k>`, and that key is dropped from the
/// result.
#[expect(
    clippy::module_name_repetitions,
    reason = "merge_map is the map-merge strategy; the name reads at call sites and is the public verb."
)]
#[expect(
    clippy::type_complexity,
    reason = "`Contributions<BTreeMap<..>>` is the natural keyed-contributions input shape."
)]
pub fn merge_map<V>(
    key_prefix: &str,
    contributions: Contributions<BTreeMap<String, V>>,
    render: impl Fn(&V) -> String,
    conflicts: &mut Vec<ConflictEntry>,
) -> BTreeMap<String, V>
where
    V: PartialEq + Clone,
{
    merge_map_by(key_prefix, contributions, V::clone, render, conflicts)
}

/// Merge a set of `key -> value` maps, comparing entries via `project`.
///
/// Like [`merge_map`], but agreement is decided on the projected value — the
/// way engines exclude the policy-authored message from the comparison (two
/// policies asserting the same semantic value with different messages agree;
/// the first entry wins).
#[expect(
    clippy::module_name_repetitions,
    reason = "merge_map_by is the projected variant of merge_map; the names pair at call sites."
)]
#[expect(
    clippy::type_complexity,
    reason = "grouping contributions by map key needs `Contributions<BTreeMap<..>>` in and `BTreeMap<_, Contributions<V>>` internally; both are the natural shapes."
)]
pub fn merge_map_by<V, P>(
    key_prefix: &str,
    contributions: Contributions<BTreeMap<String, V>>,
    project: impl Fn(&V) -> P,
    render: impl Fn(&V) -> String,
    conflicts: &mut Vec<ConflictEntry>,
) -> BTreeMap<String, V>
where
    P: PartialEq,
{
    let mut by_key: BTreeMap<String, Contributions<V>> = BTreeMap::new();
    for (prov, map) in contributions {
        for (k, v) in map {
            by_key.entry(k).or_default().push((prov.clone(), v));
        }
    }
    let mut out: BTreeMap<String, V> = BTreeMap::new();
    for (k, entries) in by_key {
        let full_key = format!("{key_prefix}.{k}");
        let mut iter = entries.into_iter();
        let Some((first_prov, first_val)) = iter.next() else {
            continue;
        };
        let mut contributors: Contributions<String> = vec![(first_prov, render(&first_val))];
        let mut disagree = false;
        for (prov, value) in iter {
            if project(&value) != project(&first_val) {
                disagree = true;
            }
            contributors.push((prov, render(&value)));
        }
        if disagree {
            conflicts.push(ConflictEntry {
                key: full_key,
                reason: "set-key-disagree".to_owned(),
                contributors,
            });
        } else {
            let _ = out.insert(k, first_val);
        }
    }
    out
}

/// First-wins union of string-keyed maps (used for `Excludes`-style unions
/// where the value is the policy message and carries no agreement semantics).
#[must_use]
#[expect(
    clippy::type_complexity,
    reason = "Vec<BTreeMap<String, V>> is the natural already-extracted input shape."
)]
pub fn union_first_wins<V>(maps: Vec<BTreeMap<String, V>>) -> BTreeMap<String, V> {
    let mut out: BTreeMap<String, V> = BTreeMap::new();
    for map in maps {
        for (k, v) in map {
            let _ = out.entry(k).or_insert(v);
        }
    }
    out
}

/// Ordered, deduplicated union of `(list, message)` contributions; the first
/// message wins. Pure union; never conflicts.
#[must_use]
#[expect(
    clippy::type_complexity,
    reason = "(list, message) pairs are the natural extracted assertion payload."
)]
pub fn union_string_lists(lists: Vec<(Vec<String>, Msg)>) -> (Vec<String>, Msg) {
    let mut items: Vec<String> = Vec::new();
    let mut msg: Option<Msg> = None;
    for (list, m) in lists {
        if msg.is_none() {
            msg = Some(m);
        }
        for it in list {
            if !items.iter().any(|e| e == &it) {
                items.push(it);
            }
        }
    }
    (items, msg.unwrap_or_default())
}

/// Union of `(set, message)` contributions; the first message wins.
#[must_use]
#[expect(
    clippy::type_complexity,
    reason = "(set, message) pairs are the natural extracted assertion payload."
)]
pub fn union_string_sets(sets: Vec<(BTreeSet<String>, Msg)>) -> (BTreeSet<String>, Msg) {
    let mut items: BTreeSet<String> = BTreeSet::new();
    let mut msg: Option<Msg> = None;
    for (set, m) in sets {
        if msg.is_none() {
            msg = Some(m);
        }
        items.extend(set);
    }
    (items, msg.unwrap_or_default())
}

/// A keyed map of `(value, message)` entries, as `Contains`-style assertions
/// carry them.
pub type KeyedEntries<S, M> = BTreeMap<String, (S, M)>;

/// Semantic equality of two keyed `(value, message)` entry maps: keys and
/// values must match; the messages never participate.
#[must_use]
pub fn keyed_entries_eq<S: PartialEq, M>(a: &KeyedEntries<S, M>, b: &KeyedEntries<S, M>) -> bool {
    a.len() == b.len()
        && a.iter()
            .zip(b)
            .all(|((ka, (sa, _)), (kb, (sb, _)))| ka == kb && sa == sb)
}

/// Union the contribution lists of two `MergedAssertion`s for the same key.
#[must_use]
pub fn union_assertion<A>(
    mut a: MergedAssertion<A>,
    mut b: MergedAssertion<A>,
) -> MergedAssertion<A> {
    a.contributions.append(&mut b.contributions);
    a
}

/// Union two optional single-assertion fields by concatenating contributions.
#[must_use]
pub fn union_optional<A>(
    a: Option<MergedAssertion<A>>,
    b: Option<MergedAssertion<A>>,
) -> Option<MergedAssertion<A>> {
    match (a, b) {
        (Some(x), Some(y)) => Some(union_assertion(x, y)),
        (x, None) => x,
        (None, y) => y,
    }
}

/// Union one field across requirements.
///
/// For a shared key, the two `MergedAssertion`s' contribution lists concatenate.
pub fn union_field<K, A>(into: &mut AssertionMap<K, A>, other: AssertionMap<K, A>)
where
    K: Ord,
{
    for (k, merged) in other {
        match into.remove(&k) {
            Some(existing) => {
                let _ = into.insert(k, union_assertion(existing, merged));
            }
            None => {
                let _ = into.insert(k, merged);
            }
        }
    }
}

/// Resolve one field after union.
///
/// Per key, run [`Resolve::resolve`] over its contributions; a resolved value is
/// re-paired with every contributing provenance (so the apply phase keeps full
/// attribution); a conflict drops the key. `key_of` maps a field key to its
/// in-file path prefix.
pub fn resolve_field<K, A>(
    field: AssertionMap<K, A>,
    key_of: impl Fn(&K) -> String,
    conflicts: &mut Vec<ConflictEntry>,
) -> AssertionMap<K, A>
where
    K: Ord,
    A: Resolve,
{
    let mut out: AssertionMap<K, A> = BTreeMap::new();
    for (k, merged) in field {
        let provenances: Vec<Provenance> = merged
            .contributions
            .iter()
            .map(|(p, _)| p.clone())
            .collect();
        let key = key_of(&k);
        if let Some(resolved) = A::resolve(&key, merged.contributions, conflicts) {
            let contributions = provenances
                .into_iter()
                .map(|p| (p, resolved.clone()))
                .collect();
            let _ = out.insert(k, MergedAssertion { contributions });
        }
    }
    out
}

/// Resolve an optional single-assertion field (e.g. `features`).
pub fn resolve_optional<A>(
    key: &str,
    field: Option<MergedAssertion<A>>,
    conflicts: &mut Vec<ConflictEntry>,
) -> Option<MergedAssertion<A>>
where
    A: Resolve,
{
    let merged = field?;
    let provenances: Vec<Provenance> = merged
        .contributions
        .iter()
        .map(|(p, _)| p.clone())
        .collect();
    let resolved = A::resolve(key, merged.contributions, conflicts)?;
    let contributions = provenances
        .into_iter()
        .map(|p| (p, resolved.clone()))
        .collect();
    Some(MergedAssertion { contributions })
}
