//! `[package].<field>` / `[workspace.package].<field>` assertions.

#![cfg_attr(
    not(test),
    expect(
        clippy::missing_docs_in_private_items,
        reason = "Private package-field composition helpers are internal requirement steps."
    )
)]
#![expect(
    clippy::indexing_slicing,
    clippy::option_option,
    clippy::type_complexity,
    clippy::wildcard_enum_match_arm,
    reason = "Package-field composition uses three-state resolution and closed local assertion enums."
)]

use std::collections::BTreeSet;

use aqc_file_engine_core::{
    ConfigScalar, ConflictEntry, ListRequirements, OnEmpty, OnEmptyClass, Provenance, Resolve,
    ResolvedListRequirements, ResolvedRequirement, parse_version_tuple, resolve_list,
};

use super::helpers::{intersect_string_sets_with_message, push_debug_conflict};

/// What must hold about a single `[package].<field>` (or
/// `[workspace.package].<field>`). One enum covers scalar and list-shaped
/// fields; the engine knows from the field key which shape applies.
///
/// Equality (and therefore merge agreement) compares the semantic value only;
/// the policy-authored message never participates.
#[derive(Debug, Clone)]
pub enum PackageFieldAssertion {
    /// The field equals this value (string, integer, or bool form).
    Equals(ConfigScalar, String),
    /// Version-ordered floor (rust-version, version, edition).
    AtLeastVersion(String, String),
    /// The field's value is one of these (check-only: the engine cannot pick).
    OneOf(BTreeSet<String>, String),
    /// List product requirements for this field.
    List(ListRequirements),
    /// The field uses workspace inheritance: `<field>.workspace = true`.
    InheritsWorkspace(String),
    /// The field is set, to anything (check-only).
    Present(String),
    /// The field is not set.
    Absent(String),
}

/// Resolved package/workspace-package field assertion.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResolvedPackageFieldAssertion {
    /// The field equals this value.
    Equals(ConfigScalar, String),
    /// Version-ordered floor.
    AtLeastVersion(String, String),
    /// The field's value is one of these.
    OneOf(BTreeSet<String>, String),
    /// Resolved list product requirements for this field.
    List(ResolvedListRequirements),
    /// The field uses workspace inheritance.
    InheritsWorkspace(String),
    /// The field is set, to anything.
    Present(String),
    /// The field is not set.
    Absent(String),
}

/// Semantic equality: messages excluded.
impl PartialEq for PackageFieldAssertion {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Equals(a, _), Self::Equals(b, _)) => a == b,
            (Self::AtLeastVersion(a, _), Self::AtLeastVersion(b, _)) => a == b,
            (Self::OneOf(a, _), Self::OneOf(b, _)) => a == b,
            (Self::List(a), Self::List(b)) => a == b,
            (Self::InheritsWorkspace(_), Self::InheritsWorkspace(_))
            | (Self::Present(_), Self::Present(_))
            | (Self::Absent(_), Self::Absent(_)) => true,
            _ => false,
        }
    }
}

impl Resolve for PackageFieldAssertion {
    type Merged = ResolvedPackageFieldAssertion;

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
        if let Some(resolved) = resolve_absent_or_inheritance(key, &items, conflicts) {
            return resolved.map(|merged| ResolvedRequirement {
                merged,
                collected: items,
            });
        }

        resolve_scalar_assertions(key, items, conflicts)
    }
}

impl OnEmptyClass for PackageFieldAssertion {
    fn on_empty(&self) -> OnEmpty {
        match self {
            Self::Equals(..)
            | Self::AtLeastVersion(..)
            | Self::List(..)
            | Self::InheritsWorkspace(..)
            | Self::Absent(..) => OnEmpty::Writes,
            Self::OneOf(..) | Self::Present(..) => OnEmpty::ChecksOnly,
        }
    }
}

impl OnEmptyClass for ResolvedPackageFieldAssertion {
    fn on_empty(&self) -> OnEmpty {
        match self {
            Self::Equals(..)
            | Self::AtLeastVersion(..)
            | Self::List(..)
            | Self::InheritsWorkspace(..)
            | Self::Absent(..) => OnEmpty::Writes,
            Self::OneOf(..) | Self::Present(..) => OnEmpty::ChecksOnly,
        }
    }
}

fn resolve_list_or_present(
    key: &str,
    items: &[(Provenance, PackageFieldAssertion)],
    conflicts: &mut Vec<ConflictEntry>,
) -> Option<Option<ResolvedPackageFieldAssertion>> {
    let has_list = items
        .iter()
        .any(|(_, a)| matches!(a, PackageFieldAssertion::List(_)));
    if !has_list {
        return None;
    }
    if !items.iter().all(|(_, a)| {
        matches!(
            a,
            PackageFieldAssertion::List(_) | PackageFieldAssertion::Present(_)
        )
    }) {
        push_conflict(key, items, conflicts);
        return Some(None);
    }
    let list_items = items
        .iter()
        .filter_map(|(prov, assertion)| match assertion {
            PackageFieldAssertion::List(list) => Some((prov.clone(), list.clone())),
            _ => None,
        })
        .collect();
    let resolved = resolve_list(key, list_items, conflicts);
    Some(Some(ResolvedPackageFieldAssertion::List(resolved)))
}

fn resolve_absent_or_inheritance(
    key: &str,
    items: &[(Provenance, PackageFieldAssertion)],
    conflicts: &mut Vec<ConflictEntry>,
) -> Option<Option<ResolvedPackageFieldAssertion>> {
    if items
        .iter()
        .any(|(_, a)| matches!(a, PackageFieldAssertion::Absent(_)))
    {
        if items
            .iter()
            .all(|(_, a)| matches!(a, PackageFieldAssertion::Absent(_)))
        {
            return Some(Some(ResolvedPackageFieldAssertion::Absent(first_msg(
                items,
            ))));
        }
        push_conflict(key, items, conflicts);
        return Some(None);
    }
    let has_inheritance = items
        .iter()
        .any(|(_, a)| matches!(a, PackageFieldAssertion::InheritsWorkspace(_)));
    if !has_inheritance {
        return None;
    }
    if items.iter().all(|(_, a)| {
        matches!(
            a,
            PackageFieldAssertion::InheritsWorkspace(_) | PackageFieldAssertion::Present(_)
        )
    }) {
        return Some(Some(ResolvedPackageFieldAssertion::InheritsWorkspace(
            inheritance_msg(items),
        )));
    }
    push_conflict(key, items, conflicts);
    Some(None)
}

fn resolve_scalar_assertions(
    key: &str,
    items: Vec<(Provenance, PackageFieldAssertion)>,
    conflicts: &mut Vec<ConflictEntry>,
) -> Option<ResolvedRequirement<ResolvedPackageFieldAssertion, PackageFieldAssertion>> {
    let equals = items
        .iter()
        .filter_map(|(_, assertion)| match assertion {
            PackageFieldAssertion::Equals(value, msg) => Some((value.clone(), msg.clone())),
            _ => None,
        })
        .collect::<Vec<_>>();
    let floors = items
        .iter()
        .filter_map(|(_, assertion)| {
            if let PackageFieldAssertion::AtLeastVersion(version, msg) = assertion {
                Some((version.clone(), msg.clone()))
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    let oneofs = items
        .iter()
        .filter_map(|(_, assertion)| match assertion {
            PackageFieldAssertion::OneOf(values, msg) => Some((values.clone(), msg.clone())),
            _ => None,
        })
        .collect::<Vec<_>>();

    if equals.windows(2).any(|pair| pair[0].0 != pair[1].0) {
        push_conflict(key, &items, conflicts);
        return None;
    }
    let oneof = intersect_string_sets_with_message(oneofs);
    let floor = strongest_floor(floors);

    let merged = if let Some((value, msg)) = equals.first() {
        let value_text = scalar_text(value);
        if oneof
            .as_ref()
            .is_some_and(|(allowed, _)| !allowed.contains(&value_text))
            || floor
                .as_ref()
                .is_some_and(|(min, _)| parse_version_tuple(&value_text) < parse_version_tuple(min))
        {
            push_conflict(key, &items, conflicts);
            return None;
        }
        ResolvedPackageFieldAssertion::Equals(value.clone(), msg.clone())
    } else if let Some((min, min_msg)) = floor {
        if let Some((allowed, allowed_msg)) = oneof {
            let filtered = allowed
                .into_iter()
                .filter(|value| parse_version_tuple(value) >= parse_version_tuple(&min))
                .collect::<BTreeSet<_>>();
            if filtered.is_empty() {
                push_conflict(key, &items, conflicts);
                return None;
            }
            ResolvedPackageFieldAssertion::OneOf(filtered, allowed_msg)
        } else {
            ResolvedPackageFieldAssertion::AtLeastVersion(min, min_msg)
        }
    } else if let Some((allowed, msg)) = oneof {
        if allowed.is_empty() {
            push_conflict(key, &items, conflicts);
            return None;
        }
        ResolvedPackageFieldAssertion::OneOf(allowed, msg)
    } else {
        ResolvedPackageFieldAssertion::Present(first_msg(&items))
    };

    Some(ResolvedRequirement {
        merged,
        collected: items,
    })
}

fn strongest_floor(floors: Vec<(String, String)>) -> Option<(String, String)> {
    floors
        .into_iter()
        .max_by(|(a, _), (b, _)| parse_version_tuple(a).cmp(&parse_version_tuple(b)))
}

fn first_msg(items: &[(Provenance, PackageFieldAssertion)]) -> String {
    items
        .iter()
        .find_map(|(_, assertion)| match assertion {
            PackageFieldAssertion::Equals(_, msg)
            | PackageFieldAssertion::AtLeastVersion(_, msg)
            | PackageFieldAssertion::OneOf(_, msg)
            | PackageFieldAssertion::InheritsWorkspace(msg)
            | PackageFieldAssertion::Present(msg)
            | PackageFieldAssertion::Absent(msg) => Some(msg.clone()),
            PackageFieldAssertion::List(_) => None,
        })
        .unwrap_or_default()
}

fn inheritance_msg(items: &[(Provenance, PackageFieldAssertion)]) -> String {
    items
        .iter()
        .find_map(|(_, assertion)| match assertion {
            PackageFieldAssertion::InheritsWorkspace(msg) => Some(msg.clone()),
            _ => None,
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
    items: &[(Provenance, PackageFieldAssertion)],
    conflicts: &mut Vec<ConflictEntry>,
) {
    push_debug_conflict(key, "scalar-disagree", items, conflicts);
}
