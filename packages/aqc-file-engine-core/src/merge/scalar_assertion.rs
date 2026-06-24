//! Generic scalar assertion merge implementation.

use std::cmp::Ordering;
use std::collections::BTreeSet;

use super::{
    ConflictEntry, Provenanced, Resolve, ResolvedAssertionOption, ResolvedRequirement,
    ScalarAssertion, ScalarValue,
};
use crate::types::{OnEmpty, OnEmptyClass};

/// One provenance-tagged scalar assertion.
type ScalarInput<T> = Provenanced<ScalarAssertion<T>>;
/// Scalar assertions grouped for one file key.
type ScalarInputs<T> = Vec<ScalarInput<T>>;
/// Borrowed scalar assertions grouped for one file key.
type ScalarInputSlice<'a, T> = &'a [ScalarInput<T>];
/// A scalar value paired with its user-facing message.
type ScalarBound<T> = (T, String);
/// Optional scalar bound or exact value.
type ScalarBoundOption<T> = Option<ScalarBound<T>>;
/// Fallible scalar-bound composition result.
type ScalarBoundResolution<T> = Option<ScalarBoundOption<T>>;
/// A one-of set paired with its user-facing message.
type ScalarOneOf<T> = (BTreeSet<T>, String);
/// Optional one-of assertion after intersection.
type ScalarOneOfOption<T> = Option<ScalarOneOf<T>>;
/// Optional borrowed scalar bound.
type ScalarBoundRef<'a, T> = Option<&'a ScalarBound<T>>;
/// Optional scalar assertion produced by a partial resolver.
type ScalarOptionalAssertion<T> = Option<ScalarAssertion<T>>;
/// Fallible optional scalar assertion resolution.
type ScalarOptionalAssertionResolution<T> = Option<ScalarOptionalAssertion<T>>;

impl<T> Resolve for ScalarAssertion<T>
where
    T: ScalarValue,
{
    type Merged = Self;

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

/// Resolves generic scalar assertions for one file key.
fn resolve_scalar_assertions<T>(
    key: &str,
    items: ScalarInputs<T>,
    conflicts: &mut Vec<ConflictEntry>,
) -> ResolvedAssertionOption<ScalarAssertion<T>>
where
    T: ScalarValue,
{
    if items.is_empty() {
        return None;
    }

    reject_unsupported_ordering(key, &items, conflicts)?;

    if let Some(merged) = resolve_absent_scalar_assertions(key, &items, conflicts)? {
        return Some(ResolvedRequirement {
            merged,
            collected: items,
        });
    }

    let equals = collect_scalar_equals(&items);
    let oneof = intersect_scalar_oneofs(collect_scalar_oneofs(&items));
    let floor = strongest_scalar_floor(key, &items, conflicts)?;
    let ceiling = strongest_scalar_ceiling(key, &items, conflicts)?;
    let merged =
        merge_present_scalar_assertions(key, &items, &equals, oneof, floor, ceiling, conflicts)?;

    Some(ResolvedRequirement {
        merged,
        collected: items,
    })
}

/// Resolves absent-only scalar groups and rejects absent/present mixtures.
fn resolve_absent_scalar_assertions<T>(
    key: &str,
    items: ScalarInputSlice<'_, T>,
    conflicts: &mut Vec<ConflictEntry>,
) -> ScalarOptionalAssertionResolution<T>
where
    T: ScalarValue,
{
    if !items
        .iter()
        .any(|(_, assertion)| matches!(assertion, ScalarAssertion::Absent(_)))
    {
        return Some(None);
    }
    if items
        .iter()
        .all(|(_, assertion)| matches!(assertion, ScalarAssertion::Absent(_)))
    {
        return Some(Some(ScalarAssertion::Absent(first_scalar_msg(items))));
    }
    push_scalar_conflict(key, "scalar-disagree", items, conflicts);
    None
}

/// Extracts exact scalar assertions.
fn collect_scalar_equals<T>(items: ScalarInputSlice<'_, T>) -> Vec<ScalarBound<T>>
where
    T: ScalarValue,
{
    items
        .iter()
        .filter_map(|(_, assertion)| match assertion {
            ScalarAssertion::Equals(value, msg) => Some((value.clone(), msg.clone())),
            ScalarAssertion::AtLeast(..)
            | ScalarAssertion::AtMost(..)
            | ScalarAssertion::Range(..)
            | ScalarAssertion::OneOf(..)
            | ScalarAssertion::Present(_)
            | ScalarAssertion::Absent(_) => None,
        })
        .collect()
}

/// Extracts one-of scalar assertions.
fn collect_scalar_oneofs<T>(items: ScalarInputSlice<'_, T>) -> Vec<ScalarOneOf<T>>
where
    T: ScalarValue,
{
    items
        .iter()
        .filter_map(|(_, assertion)| match assertion {
            ScalarAssertion::OneOf(values, msg) => Some((values.clone(), msg.clone())),
            ScalarAssertion::Equals(..)
            | ScalarAssertion::AtLeast(..)
            | ScalarAssertion::AtMost(..)
            | ScalarAssertion::Range(..)
            | ScalarAssertion::Present(_)
            | ScalarAssertion::Absent(_) => None,
        })
        .collect()
}

/// Composes present scalar assertions after absent handling.
fn merge_present_scalar_assertions<T>(
    key: &str,
    items: ScalarInputSlice<'_, T>,
    equals: &[ScalarBound<T>],
    oneof: ScalarOneOfOption<T>,
    floor: ScalarBoundOption<T>,
    ceiling: ScalarBoundOption<T>,
    conflicts: &mut Vec<ConflictEntry>,
) -> Option<ScalarAssertion<T>>
where
    T: ScalarValue,
{
    if equals
        .windows(2)
        .any(|pair| matches!(pair, [left, right] if left.0 != right.0))
    {
        push_scalar_conflict(key, "scalar-disagree", items, conflicts);
        return None;
    }
    if let Some((value, msg)) = equals.first() {
        if oneof
            .as_ref()
            .is_some_and(|(allowed, _)| !allowed.contains(value))
            || bound_rejects_value(
                key,
                items,
                value,
                floor.as_ref(),
                ceiling.as_ref(),
                conflicts,
            )?
        {
            push_scalar_conflict(key, "scalar-disagree", items, conflicts);
            return None;
        }
        return Some(ScalarAssertion::Equals(value.clone(), msg.clone()));
    }
    if let Some((mut allowed, allowed_msg)) = oneof {
        filter_allowed_by_bounds(
            key,
            items,
            &mut allowed,
            floor.as_ref(),
            ceiling.as_ref(),
            conflicts,
        )?;
        if allowed.is_empty() {
            push_scalar_conflict(key, "scalar-disagree", items, conflicts);
            return None;
        }
        return Some(ScalarAssertion::OneOf(allowed, allowed_msg));
    }
    scalar_from_bounds(key, items, floor, ceiling, conflicts)
}

/// Intersects one-of scalar assertions while preserving the first message.
fn intersect_scalar_oneofs<T: Ord>(oneofs: Vec<ScalarOneOf<T>>) -> ScalarOneOfOption<T> {
    let mut iter = oneofs.into_iter();
    let (mut out, msg) = iter.next()?;
    for (next, _) in iter {
        out.retain(|item| next.contains(item));
    }
    Some((out, msg))
}

/// Rejects ordering assertions for values that cannot be ordered.
fn reject_unsupported_ordering<T>(
    key: &str,
    items: ScalarInputSlice<'_, T>,
    conflicts: &mut Vec<ConflictEntry>,
) -> Option<()>
where
    T: ScalarValue,
{
    for (_, assertion) in items {
        let (ScalarAssertion::AtLeast(value, _)
        | ScalarAssertion::AtMost(value, _)
        | ScalarAssertion::Range(value, _, _)) = assertion
        else {
            continue;
        };
        if value.compare_for_order(value).is_none() {
            push_scalar_conflict(key, "scalar-order-unsupported", items, conflicts);
            return None;
        }
    }
    Some(())
}

/// Selects the strongest lower bound from at-least and range assertions.
fn strongest_scalar_floor<T>(
    key: &str,
    items: ScalarInputSlice<'_, T>,
    conflicts: &mut Vec<ConflictEntry>,
) -> ScalarBoundResolution<T>
where
    T: ScalarValue,
{
    let mut out: ScalarBoundOption<T> = None;
    for (_, assertion) in items {
        let next = match assertion {
            ScalarAssertion::AtLeast(value, msg) | ScalarAssertion::Range(value, _, msg) => {
                Some((value.clone(), msg.clone()))
            }
            ScalarAssertion::Equals(..)
            | ScalarAssertion::AtMost(..)
            | ScalarAssertion::OneOf(..)
            | ScalarAssertion::Present(_)
            | ScalarAssertion::Absent(_) => None,
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

/// Selects the strongest upper bound from at-most and range assertions.
fn strongest_scalar_ceiling<T>(
    key: &str,
    items: ScalarInputSlice<'_, T>,
    conflicts: &mut Vec<ConflictEntry>,
) -> ScalarBoundResolution<T>
where
    T: ScalarValue,
{
    let mut out: ScalarBoundOption<T> = None;
    for (_, assertion) in items {
        let next = match assertion {
            ScalarAssertion::AtMost(value, msg) | ScalarAssertion::Range(_, value, msg) => {
                Some((value.clone(), msg.clone()))
            }
            ScalarAssertion::Equals(..)
            | ScalarAssertion::AtLeast(..)
            | ScalarAssertion::OneOf(..)
            | ScalarAssertion::Present(_)
            | ScalarAssertion::Absent(_) => None,
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

/// Returns whether a scalar value violates a resolved lower or upper bound.
fn bound_rejects_value<T>(
    key: &str,
    items: ScalarInputSlice<'_, T>,
    value: &T,
    floor: ScalarBoundRef<'_, T>,
    ceiling: ScalarBoundRef<'_, T>,
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

/// Removes one-of values that violate resolved scalar bounds.
fn filter_allowed_by_bounds<T>(
    key: &str,
    items: ScalarInputSlice<'_, T>,
    allowed: &mut BTreeSet<T>,
    floor: ScalarBoundRef<'_, T>,
    ceiling: ScalarBoundRef<'_, T>,
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

/// Composes the scalar assertion represented by resolved bounds.
fn scalar_from_bounds<T>(
    key: &str,
    items: ScalarInputSlice<'_, T>,
    floor: ScalarBoundOption<T>,
    ceiling: ScalarBoundOption<T>,
    conflicts: &mut Vec<ConflictEntry>,
) -> Option<ScalarAssertion<T>>
where
    T: ScalarValue,
{
    match (floor, ceiling) {
        (Some((min, min_msg)), Some((max, max_msg))) => {
            if compare_order(key, items, &min, &max, conflicts)? == Ordering::Greater {
                push_scalar_conflict(key, "scalar-disagree", items, conflicts);
                return None;
            }
            Some(ScalarAssertion::Range(
                min,
                max,
                format!("{min_msg}; {max_msg}"),
            ))
        }
        (Some((min, msg)), None) => Some(ScalarAssertion::AtLeast(min, msg)),
        (None, Some((max, msg))) => Some(ScalarAssertion::AtMost(max, msg)),
        (None, None) => Some(ScalarAssertion::Present(first_scalar_msg(items))),
    }
}

/// Compares ordered scalar values or reports an unsupported-order conflict.
fn compare_order<T>(
    key: &str,
    items: ScalarInputSlice<'_, T>,
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

/// Returns the first scalar assertion message in a group.
fn first_scalar_msg<T>(items: ScalarInputSlice<'_, T>) -> String {
    items
        .iter()
        .map(|(_, assertion)| assertion.message().to_owned())
        .next()
        .unwrap_or_default()
}

/// Records a scalar merge conflict with rendered contributors.
fn push_scalar_conflict<T>(
    key: &str,
    reason: &str,
    items: ScalarInputSlice<'_, T>,
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

/// Renders a scalar assertion for conflict attribution.
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
