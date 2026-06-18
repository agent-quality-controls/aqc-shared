//! `[workspace].<key>` assertions (resolver, members, exclude, default-members).

#![cfg_attr(
    not(test),
    expect(
        clippy::missing_docs_in_private_items,
        reason = "Private workspace-field composition helpers are internal requirement steps."
    )
)]
#![expect(
    clippy::indexing_slicing,
    clippy::option_option,
    clippy::type_complexity,
    clippy::wildcard_enum_match_arm,
    reason = "Workspace-field composition uses three-state resolution and closed local assertion enums."
)]

use std::collections::BTreeSet;

use aqc_file_engine_core::{
    ConfigScalar, ConflictEntry, ListRequirements, OnEmpty, OnEmptyClass, Provenance, Resolve,
    ResolvedListRequirements, ResolvedRequirement, resolve_list,
};

use super::helpers::{intersect_string_sets_with_message, push_debug_conflict};

/// What must hold about a direct `[workspace]` key.
///
/// Equality (and therefore merge agreement) compares the semantic value only;
/// the policy-authored message never participates.
#[derive(Debug, Clone)]
pub enum WorkspaceFieldAssertion {
    /// The key equals this value (`resolver = "3"`).
    Equals(ConfigScalar, String),
    /// The key's value is one of these (check-only).
    OneOf(BTreeSet<String>, String),
    /// List product requirements for this key.
    List(ListRequirements),
    /// The key is set, to anything (check-only).
    Present(String),
    /// The key is not set.
    Absent(String),
}

/// Resolved workspace field assertion.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResolvedWorkspaceFieldAssertion {
    /// The key equals this value.
    Equals(ConfigScalar, String),
    /// The key's value is one of these.
    OneOf(BTreeSet<String>, String),
    /// Resolved list product requirements for this key.
    List(ResolvedListRequirements),
    /// The key is set, to anything.
    Present(String),
    /// The key is not set.
    Absent(String),
}

/// Semantic equality: messages excluded.
impl PartialEq for WorkspaceFieldAssertion {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Equals(a, _), Self::Equals(b, _)) => a == b,
            (Self::OneOf(a, _), Self::OneOf(b, _)) => a == b,
            (Self::List(a), Self::List(b)) => a == b,
            (Self::Present(_), Self::Present(_)) | (Self::Absent(_), Self::Absent(_)) => true,
            _ => false,
        }
    }
}

impl Resolve for WorkspaceFieldAssertion {
    type Merged = ResolvedWorkspaceFieldAssertion;

    fn resolve(
        key: &str,
        items: Vec<(Provenance, Self)>,
        conflicts: &mut Vec<ConflictEntry>,
    ) -> Option<ResolvedRequirement<Self::Merged, Self>> {
        if let Some(resolved) = resolve_list_or_present(key, &items, conflicts) {
            return resolved.map(|merged| ResolvedRequirement {
                merged,
                collected: items,
            });
        }
        if items.iter().any(|(_, a)| matches!(a, Self::Absent(_))) {
            if items.iter().all(|(_, a)| matches!(a, Self::Absent(_))) {
                return Some(ResolvedRequirement {
                    merged: ResolvedWorkspaceFieldAssertion::Absent(first_msg(&items)),
                    collected: items,
                });
            }
            push_conflict(key, &items, conflicts);
            return None;
        }
        resolve_scalar_assertions(key, items, conflicts)
    }
}

impl OnEmptyClass for WorkspaceFieldAssertion {
    fn on_empty(&self) -> OnEmpty {
        match self {
            Self::Equals(..) | Self::List(..) | Self::Absent(..) => OnEmpty::Writes,
            Self::OneOf(..) | Self::Present(..) => OnEmpty::ChecksOnly,
        }
    }
}

impl OnEmptyClass for ResolvedWorkspaceFieldAssertion {
    fn on_empty(&self) -> OnEmpty {
        match self {
            Self::Equals(..) | Self::List(..) | Self::Absent(..) => OnEmpty::Writes,
            Self::OneOf(..) | Self::Present(..) => OnEmpty::ChecksOnly,
        }
    }
}

fn resolve_list_or_present(
    key: &str,
    items: &[(Provenance, WorkspaceFieldAssertion)],
    conflicts: &mut Vec<ConflictEntry>,
) -> Option<Option<ResolvedWorkspaceFieldAssertion>> {
    let has_list = items
        .iter()
        .any(|(_, a)| matches!(a, WorkspaceFieldAssertion::List(_)));
    if !has_list {
        return None;
    }
    if !items.iter().all(|(_, a)| {
        matches!(
            a,
            WorkspaceFieldAssertion::List(_) | WorkspaceFieldAssertion::Present(_)
        )
    }) {
        push_conflict(key, items, conflicts);
        return Some(None);
    }
    let list_items = items
        .iter()
        .filter_map(|(prov, assertion)| match assertion {
            WorkspaceFieldAssertion::List(list) => Some((prov.clone(), list.clone())),
            _ => None,
        })
        .collect();
    Some(Some(ResolvedWorkspaceFieldAssertion::List(resolve_list(
        key, list_items, conflicts,
    ))))
}

fn resolve_scalar_assertions(
    key: &str,
    items: Vec<(Provenance, WorkspaceFieldAssertion)>,
    conflicts: &mut Vec<ConflictEntry>,
) -> Option<ResolvedRequirement<ResolvedWorkspaceFieldAssertion, WorkspaceFieldAssertion>> {
    let equals = items
        .iter()
        .filter_map(|(_, assertion)| match assertion {
            WorkspaceFieldAssertion::Equals(value, msg) => Some((value.clone(), msg.clone())),
            _ => None,
        })
        .collect::<Vec<_>>();
    let oneof = intersect_string_sets_with_message(
        items
            .iter()
            .filter_map(|(_, assertion)| match assertion {
                WorkspaceFieldAssertion::OneOf(values, msg) => Some((values.clone(), msg.clone())),
                _ => None,
            })
            .collect(),
    );

    let merged = if equals.windows(2).any(|pair| pair[0].0 != pair[1].0) {
        push_conflict(key, &items, conflicts);
        return None;
    } else if let Some((value, msg)) = equals.first() {
        if oneof
            .as_ref()
            .is_some_and(|(allowed, _)| !allowed.contains(&scalar_text(value)))
        {
            push_conflict(key, &items, conflicts);
            return None;
        }
        ResolvedWorkspaceFieldAssertion::Equals(value.clone(), msg.clone())
    } else if let Some((allowed, msg)) = oneof {
        if allowed.is_empty() {
            push_conflict(key, &items, conflicts);
            return None;
        }
        ResolvedWorkspaceFieldAssertion::OneOf(allowed, msg)
    } else {
        ResolvedWorkspaceFieldAssertion::Present(first_msg(&items))
    };

    Some(ResolvedRequirement {
        merged,
        collected: items,
    })
}

fn first_msg(items: &[(Provenance, WorkspaceFieldAssertion)]) -> String {
    items
        .iter()
        .find_map(|(_, assertion)| match assertion {
            WorkspaceFieldAssertion::Equals(_, msg)
            | WorkspaceFieldAssertion::OneOf(_, msg)
            | WorkspaceFieldAssertion::Present(msg)
            | WorkspaceFieldAssertion::Absent(msg) => Some(msg.clone()),
            WorkspaceFieldAssertion::List(_) => None,
        })
        .unwrap_or_default()
}

fn scalar_text(value: &ConfigScalar) -> String {
    match value {
        ConfigScalar::Str(value) => value.clone(),
        ConfigScalar::Int(value) => value.to_string(),
        ConfigScalar::Bool(value) => value.to_string(),
    }
}

fn push_conflict(
    key: &str,
    items: &[(Provenance, WorkspaceFieldAssertion)],
    conflicts: &mut Vec<ConflictEntry>,
) {
    push_debug_conflict(key, "scalar-disagree", items, conflicts);
}
