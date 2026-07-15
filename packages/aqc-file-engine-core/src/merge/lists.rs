//! List requirement merge functions.

use std::collections::BTreeSet;

use super::{
    ConflictEntry, Contributor, ExactInput, ListInput, ListRequirements, MemberInputs,
    ResolvedExactList, ResolvedListRequirements, ResolvedRequirement, ResolvedStringMembers,
    resolve_all_equal, sort_provenanced,
};
use crate::{FindingKey, types::Provenance};

pub fn resolve_list<Key>(
    key: &Key,
    mut items: Vec<ListInput>,
    conflicts: &mut Vec<ConflictEntry>,
) -> ResolvedListRequirements
where
    Key: FindingKey + ?Sized,
{
    sort_provenanced(&mut items);
    let mut grouped = ListGroups::default();
    collect_list_groups(items, &mut grouped);

    let resolved_contains = resolve_members(grouped.contains);
    let resolved_excludes = resolve_members(grouped.excludes);

    report_contains_excludes_conflicts(key, &resolved_contains, &resolved_excludes, conflicts);

    let exact = resolve_exact_list(&key.key(), grouped.exact_items, conflicts);
    if let Some(exact) = &exact {
        report_exact_conflicts(
            key,
            exact,
            &resolved_contains,
            &resolved_excludes,
            conflicts,
        );
    }

    ResolvedListRequirements {
        contains: resolved_contains,
        excludes: resolved_excludes,
        exact,
    }
}

pub fn resolve_exact_list(
    key: &str,
    items: Vec<ExactInput>,
    conflicts: &mut Vec<ConflictEntry>,
) -> Option<ResolvedExactList> {
    let resolved = resolve_all_equal(
        key,
        "exact-mismatch",
        items,
        |(list, _)| format!("exact [{}]", list.join(", ")),
        conflicts,
    )?;
    Some(ResolvedRequirement {
        merged: resolved.merged.0,
        collected: resolved.collected,
    })
}

#[must_use]
pub fn render_list_requirement(requirements: &ListRequirements) -> String {
    let mut parts = Vec::new();
    if !requirements.contains.is_empty() {
        parts.push(format!(
            "contains [{}]",
            requirements
                .contains
                .keys()
                .cloned()
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    if !requirements.excludes.is_empty() {
        parts.push(format!(
            "excludes [{}]",
            requirements
                .excludes
                .keys()
                .cloned()
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    if let Some((items, _)) = &requirements.exact {
        parts.push(format!("exact [{}]", items.join(", ")));
    }
    if parts.is_empty() {
        "list".to_owned()
    } else {
        format!("list {}", parts.join("; "))
    }
}

#[derive(Default)]
/// List assertions grouped by assertion kind.
struct ListGroups {
    /// Required list members.
    contains: MemberInputs,
    /// Forbidden list members.
    excludes: MemberInputs,
    /// Exact-list assertions.
    exact_items: Vec<ExactInput>,
}

/// Group list assertions by assertion kind.
fn collect_list_groups(items: Vec<ListInput>, grouped: &mut ListGroups) {
    for (prov, list) in items {
        for (name, msg) in list.contains {
            grouped
                .contains
                .entry(name)
                .or_default()
                .push((prov.clone(), msg));
        }
        for (name, msg) in list.excludes {
            grouped
                .excludes
                .entry(name)
                .or_default()
                .push((prov.clone(), msg));
        }
        if let Some(exact) = list.exact {
            grouped.exact_items.push((prov, exact));
        }
    }
}

/// Resolve contains/excludes members with attribution.
fn resolve_members(grouped: MemberInputs) -> ResolvedStringMembers {
    let mut resolved = ResolvedStringMembers::new();
    for (name, collected) in grouped {
        let _ = resolved.insert(
            name,
            ResolvedRequirement {
                merged: (),
                collected,
            },
        );
    }
    resolved
}

/// Report members that are both contained and excluded.
fn report_contains_excludes_conflicts<Key>(
    key: &Key,
    resolved_contains: &ResolvedStringMembers,
    resolved_excludes: &ResolvedStringMembers,
    conflicts: &mut Vec<ConflictEntry>,
) where
    Key: FindingKey + ?Sized,
{
    for name in resolved_contains.keys() {
        if let Some(exclude) = resolved_excludes.get(name) {
            let mut contributors = Vec::new();
            if let Some(include) = resolved_contains.get(name) {
                contributors.extend(
                    include
                        .collected
                        .iter()
                        .map(|(prov, _)| contains_contributor(prov)),
                );
            }
            contributors.extend(
                exclude
                    .collected
                    .iter()
                    .map(|(prov, _)| excludes_contributor(prov)),
            );
            conflicts.push(ConflictEntry {
                key: key.child_key(name),
                reason: "list-contains-and-excludes".to_owned(),
                contributors,
            });
        }
    }
}

/// Report conflicts between exact lists and member assertions.
fn report_exact_conflicts<Key>(
    key: &Key,
    exact: &ResolvedExactList,
    resolved_contains: &ResolvedStringMembers,
    resolved_excludes: &ResolvedStringMembers,
    conflicts: &mut Vec<ConflictEntry>,
) where
    Key: FindingKey + ?Sized,
{
    let allowed = exact.merged.iter().cloned().collect::<BTreeSet<_>>();
    report_missing_exact_contains(key, exact, resolved_contains, &allowed, conflicts);
    report_excluded_exact_members(key, exact, resolved_excludes, &allowed, conflicts);
}

/// Report contained members missing from an exact list.
fn report_missing_exact_contains<Key>(
    key: &Key,
    exact: &ResolvedExactList,
    resolved_contains: &ResolvedStringMembers,
    allowed: &BTreeSet<String>,
    conflicts: &mut Vec<ConflictEntry>,
) where
    Key: FindingKey + ?Sized,
{
    for (name, include) in resolved_contains {
        if allowed.contains(name) {
            continue;
        }
        let mut contributors = exact_contributors(exact);
        contributors.extend(
            include
                .collected
                .iter()
                .map(|(prov, _)| contains_contributor(prov)),
        );
        conflicts.push(ConflictEntry {
            key: key.child_key(name),
            reason: "list-exact-missing-contained-item".to_owned(),
            contributors,
        });
    }
}

/// Report excluded members present in an exact list.
fn report_excluded_exact_members<Key>(
    key: &Key,
    exact: &ResolvedExactList,
    resolved_excludes: &ResolvedStringMembers,
    allowed: &BTreeSet<String>,
    conflicts: &mut Vec<ConflictEntry>,
) where
    Key: FindingKey + ?Sized,
{
    for (name, exclude) in resolved_excludes {
        if !allowed.contains(name) {
            continue;
        }
        let mut contributors = exact_contributors(exact);
        contributors.extend(
            exclude
                .collected
                .iter()
                .map(|(prov, _)| excludes_contributor(prov)),
        );
        conflicts.push(ConflictEntry {
            key: key.child_key(name),
            reason: "list-exact-contains-excluded-item".to_owned(),
            contributors,
        });
    }
}

/// Render exact-list contributors.
fn exact_contributors(exact: &ResolvedExactList) -> Vec<Contributor> {
    exact
        .collected
        .iter()
        .map(|(prov, _)| (prov.clone(), "exact".to_owned()))
        .collect()
}

/// Render a contains contributor.
fn contains_contributor(prov: &Provenance) -> Contributor {
    (prov.clone(), "contains".to_owned())
}

/// Render an excludes contributor.
fn excludes_contributor(prov: &Provenance) -> Contributor {
    (prov.clone(), "excludes".to_owned())
}
