//! Scalar, optional, and map merge functions.

use std::cmp::Ordering;
use std::collections::BTreeSet;

use super::{
    ConflictEntry, GroupedAssertions, KeyedValueMap, MapInputs, OptionalInput, Provenanced,
    Resolve, ResolvedAssertionOption, ResolvedMap, ResolvedRequirement, ResolvedSameOption,
    ScalarAssertion, ScalarValue, VersionFloor,
};
use crate::toml_helpers::parse_version_tuple;
use crate::types::{ConfigScalar, OnEmpty, OnEmptyClass};

pub fn resolve_map<K, A>(
    input: MapInputs<K, A>,
    key_path: impl Fn(&K) -> String,
    conflicts: &mut Vec<ConflictEntry>,
) -> ResolvedMap<K, A>
where
    K: Ord + Clone,
    A: Resolve,
{
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
    input: Vec<OptionalInput<A>>,
    conflicts: &mut Vec<ConflictEntry>,
) -> ResolvedAssertionOption<A>
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
    items: Vec<Provenanced<T>>,
    render: impl Fn(&T) -> String,
    conflicts: &mut Vec<ConflictEntry>,
) -> ResolvedSameOption<T>
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
        resolve_scalar(key, items, |item| format!("{item:?}"), conflicts)
    }
}

impl<T> Resolve for ScalarAssertion<T>
where
    T: ScalarValue,
{
    type Merged = ScalarAssertion<T>;

    fn resolve(
        key: &str,
        items: Vec<Provenanced<Self>>,
        conflicts: &mut Vec<ConflictEntry>,
    ) -> ResolvedAssertionOption<Self> {
        resolve_scalar_assertions(key, items, conflicts)
    }
}

impl<T> OnEmptyClass for ScalarAssertion<T> {
    fn on_empty(&self) -> OnEmpty {
        match self {
            Self::Equals(..)
            | Self::AtLeast(..)
            | Self::AtMost(..)
            | Self::Range(..)
            | Self::Absent(..) => OnEmpty::Writes,
            Self::OneOf(..) | Self::Present(..) => OnEmpty::ChecksOnly,
        }
    }
}

fn resolve_scalar_assertions<T>(
    key: &str,
    items: Vec<Provenanced<ScalarAssertion<T>>>,
    conflicts: &mut Vec<ConflictEntry>,
) -> ResolvedAssertionOption<ScalarAssertion<T>>
where
    T: ScalarValue,
{
    if items.is_empty() {
        return None;
    }

    reject_unsupported_ordering(key, &items, conflicts)?;

    if items
        .iter()
        .any(|(_, assertion)| matches!(assertion, ScalarAssertion::Absent(_)))
    {
        if items
            .iter()
            .all(|(_, assertion)| matches!(assertion, ScalarAssertion::Absent(_)))
        {
            return Some(ResolvedRequirement {
                merged: ScalarAssertion::Absent(first_scalar_msg(&items)),
                collected: items,
            });
        }
        push_scalar_conflict(key, "scalar-disagree", &items, conflicts);
        return None;
    }

    let equals = items
        .iter()
        .filter_map(|(_, assertion)| match assertion {
            ScalarAssertion::Equals(value, msg) => Some((value.clone(), msg.clone())),
            _ => None,
        })
        .collect::<Vec<_>>();
    let oneof = intersect_scalar_oneofs(
        items
            .iter()
            .filter_map(|(_, assertion)| match assertion {
                ScalarAssertion::OneOf(values, msg) => Some((values.clone(), msg.clone())),
                _ => None,
            })
            .collect(),
    );
    let floor = strongest_scalar_floor(key, &items, conflicts)?;
    let ceiling = strongest_scalar_ceiling(key, &items, conflicts)?;

    let merged = if equals.windows(2).any(|pair| pair[0].0 != pair[1].0) {
        push_scalar_conflict(key, "scalar-disagree", &items, conflicts);
        return None;
    } else if let Some((value, msg)) = equals.first() {
        if oneof
            .as_ref()
            .is_some_and(|(allowed, _)| !allowed.contains(value))
            || bound_rejects_value(
                key,
                &items,
                value,
                floor.as_ref(),
                ceiling.as_ref(),
                conflicts,
            )?
        {
            push_scalar_conflict(key, "scalar-disagree", &items, conflicts);
            return None;
        }
        ScalarAssertion::Equals(value.clone(), msg.clone())
    } else if let Some((mut allowed, allowed_msg)) = oneof {
        filter_allowed_by_bounds(
            key,
            &items,
            &mut allowed,
            floor.as_ref(),
            ceiling.as_ref(),
            conflicts,
        )?;
        if allowed.is_empty() {
            push_scalar_conflict(key, "scalar-disagree", &items, conflicts);
            return None;
        }
        ScalarAssertion::OneOf(allowed, allowed_msg)
    } else {
        match (floor, ceiling) {
            (Some((min, min_msg)), Some((max, max_msg))) => {
                if compare_order(key, &items, &min, &max, conflicts)? == Ordering::Greater {
                    push_scalar_conflict(key, "scalar-disagree", &items, conflicts);
                    return None;
                }
                ScalarAssertion::Range(min, max, format!("{min_msg}; {max_msg}"))
            }
            (Some((min, msg)), None) => ScalarAssertion::AtLeast(min, msg),
            (None, Some((max, msg))) => ScalarAssertion::AtMost(max, msg),
            (None, None) => ScalarAssertion::Present(first_scalar_msg(&items)),
        }
    };

    Some(ResolvedRequirement {
        merged,
        collected: items,
    })
}

fn intersect_scalar_oneofs<T: Ord>(
    oneofs: Vec<(BTreeSet<T>, String)>,
) -> Option<(BTreeSet<T>, String)> {
    let mut iter = oneofs.into_iter();
    let (mut out, msg) = iter.next()?;
    for (next, _) in iter {
        out.retain(|item| next.contains(item));
    }
    Some((out, msg))
}

fn reject_unsupported_ordering<T>(
    key: &str,
    items: &[Provenanced<ScalarAssertion<T>>],
    conflicts: &mut Vec<ConflictEntry>,
) -> Option<()>
where
    T: ScalarValue,
{
    for (_, assertion) in items {
        let value = match assertion {
            ScalarAssertion::AtLeast(value, _)
            | ScalarAssertion::AtMost(value, _)
            | ScalarAssertion::Range(value, _, _) => value,
            _ => continue,
        };
        if value.compare_for_order(value).is_none() {
            push_scalar_conflict(key, "scalar-order-unsupported", items, conflicts);
            return None;
        }
    }
    Some(())
}

fn strongest_scalar_floor<T>(
    key: &str,
    items: &[Provenanced<ScalarAssertion<T>>],
    conflicts: &mut Vec<ConflictEntry>,
) -> Option<Option<(T, String)>>
where
    T: ScalarValue,
{
    let mut out: Option<(T, String)> = None;
    for (_, assertion) in items {
        let next = match assertion {
            ScalarAssertion::AtLeast(value, msg) | ScalarAssertion::Range(value, _, msg) => {
                Some((value.clone(), msg.clone()))
            }
            _ => None,
        };
        let Some(next) = next else {
            continue;
        };
        if let Some((current, _)) = &out {
            match compare_order(key, items, current, &next.0, conflicts)? {
                Ordering::Less => out = Some(next),
                Ordering::Equal | Ordering::Greater => {}
            }
        } else {
            out = Some(next);
        }
    }
    Some(out)
}

fn strongest_scalar_ceiling<T>(
    key: &str,
    items: &[Provenanced<ScalarAssertion<T>>],
    conflicts: &mut Vec<ConflictEntry>,
) -> Option<Option<(T, String)>>
where
    T: ScalarValue,
{
    let mut out: Option<(T, String)> = None;
    for (_, assertion) in items {
        let next = match assertion {
            ScalarAssertion::AtMost(value, msg) | ScalarAssertion::Range(_, value, msg) => {
                Some((value.clone(), msg.clone()))
            }
            _ => None,
        };
        let Some(next) = next else {
            continue;
        };
        if let Some((current, _)) = &out {
            match compare_order(key, items, current, &next.0, conflicts)? {
                Ordering::Greater => out = Some(next),
                Ordering::Equal | Ordering::Less => {}
            }
        } else {
            out = Some(next);
        }
    }
    Some(out)
}

fn bound_rejects_value<T>(
    key: &str,
    items: &[Provenanced<ScalarAssertion<T>>],
    value: &T,
    floor: Option<&(T, String)>,
    ceiling: Option<&(T, String)>,
    conflicts: &mut Vec<ConflictEntry>,
) -> Option<bool>
where
    T: ScalarValue,
{
    if let Some((min, _)) = floor {
        if compare_order(key, items, value, min, conflicts)? == Ordering::Less {
            return Some(true);
        }
    }
    if let Some((max, _)) = ceiling {
        if compare_order(key, items, value, max, conflicts)? == Ordering::Greater {
            return Some(true);
        }
    }
    Some(false)
}

fn filter_allowed_by_bounds<T>(
    key: &str,
    items: &[Provenanced<ScalarAssertion<T>>],
    allowed: &mut BTreeSet<T>,
    floor: Option<&(T, String)>,
    ceiling: Option<&(T, String)>,
    conflicts: &mut Vec<ConflictEntry>,
) -> Option<()>
where
    T: ScalarValue,
{
    let mut filtered = BTreeSet::new();
    for value in allowed.iter() {
        if !bound_rejects_value(key, items, value, floor, ceiling, conflicts)? {
            let _ = filtered.insert(value.clone());
        }
    }
    *allowed = filtered;
    Some(())
}

fn compare_order<T>(
    key: &str,
    items: &[Provenanced<ScalarAssertion<T>>],
    left: &T,
    right: &T,
    conflicts: &mut Vec<ConflictEntry>,
) -> Option<Ordering>
where
    T: ScalarValue,
{
    let Some(ordering) = left.compare_for_order(right) else {
        push_scalar_conflict(key, "scalar-order-unsupported", items, conflicts);
        return None;
    };
    Some(ordering)
}

fn first_scalar_msg<T>(items: &[Provenanced<ScalarAssertion<T>>]) -> String {
    items
        .iter()
        .map(|(_, assertion)| assertion.message().to_owned())
        .next()
        .unwrap_or_default()
}

fn push_scalar_conflict<T>(
    key: &str,
    reason: &str,
    items: &[Provenanced<ScalarAssertion<T>>],
    conflicts: &mut Vec<ConflictEntry>,
) where
    T: ScalarValue,
{
    conflicts.push(ConflictEntry {
        key: key.to_owned(),
        reason: reason.to_owned(),
        contributors: items
            .iter()
            .map(|(prov, assertion)| (prov.clone(), render_scalar_assertion(assertion)))
            .collect(),
    });
}

fn render_scalar_assertion<T>(assertion: &ScalarAssertion<T>) -> String
where
    T: ScalarValue,
{
    match assertion {
        ScalarAssertion::Equals(value, _) => format!("equals {}", value.render()),
        ScalarAssertion::AtLeast(value, _) => format!("at least {}", value.render()),
        ScalarAssertion::AtMost(value, _) => format!("at most {}", value.render()),
        ScalarAssertion::Range(min, max, _) => {
            format!("range {}..={}", min.render(), max.render())
        }
        ScalarAssertion::OneOf(values, _) => {
            let rendered = values.iter().map(ScalarValue::render).collect::<Vec<_>>();
            format!("one of {rendered:?}")
        }
        ScalarAssertion::Present(_) => "present".to_owned(),
        ScalarAssertion::Absent(_) => "absent".to_owned(),
    }
}
