//! `[package].<field>` / `[workspace.package].<field>` assertions.

#![cfg_attr(
    not(test),
    expect(
        clippy::missing_docs_in_private_items,
        reason = "Private package-field composition helpers are internal requirement steps."
    )
)]
#![expect(
    clippy::option_option,
    clippy::type_complexity,
    reason = "Package-field composition uses product wrappers and three-state resolution."
)]

use aqc_file_engine_core::{
    ConfigScalar, ConflictEntry, DottedVersion, ListRequirements, OnEmpty, OnEmptyClass,
    Provenance, Resolve, ResolvedListRequirements, ResolvedRequirement, ScalarAssertion,
    resolve_list,
};

use super::helpers::push_debug_conflict;

/// What must hold about a single `[package].<field>` or
/// `[workspace.package].<field>`.
#[derive(Debug, Clone)]
pub enum PackageFieldAssertion {
    Scalar(ScalarAssertion<ConfigScalar>),
    OrderedVersion(ScalarAssertion<DottedVersion>),
    List(ListRequirements),
    InheritsWorkspace(String),
}

/// Resolved package/workspace-package field assertion.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResolvedPackageFieldAssertion {
    Scalar(ScalarAssertion<ConfigScalar>),
    OrderedVersion(ScalarAssertion<DottedVersion>),
    List(ResolvedListRequirements),
    InheritsWorkspace(String),
}

impl PartialEq for PackageFieldAssertion {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Scalar(a), Self::Scalar(b)) => a == b,
            (Self::OrderedVersion(a), Self::OrderedVersion(b)) => a == b,
            (Self::List(a), Self::List(b)) => a == b,
            (Self::InheritsWorkspace(_), Self::InheritsWorkspace(_)) => true,
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
        if !items
            .iter()
            .all(|(_, assertion)| package_field_assertion_is_legal(key, assertion))
        {
            push_unsupported_conflict(key, &items, conflicts);
            return None;
        }
        if let Some(resolved) = resolve_absent(key, &items, conflicts) {
            return resolved.map(|merged| ResolvedRequirement {
                merged,
                collected: items,
            });
        }
        if let Some(resolved) = resolve_list_product(key, &items, conflicts) {
            return resolved.map(|merged| ResolvedRequirement {
                merged,
                collected: items,
            });
        }
        if let Some(resolved) = resolve_inheritance(key, &items, conflicts) {
            return resolved.map(|merged| ResolvedRequirement {
                merged,
                collected: items,
            });
        }
        if let Some(resolved) = resolve_ordered_version(key, &items, conflicts) {
            return resolved.map(|merged| ResolvedRequirement {
                merged,
                collected: items,
            });
        }

        resolve_config_scalar(key, items, conflicts)
    }
}

impl OnEmptyClass for PackageFieldAssertion {
    fn on_empty(&self) -> OnEmpty {
        match self {
            Self::Scalar(assertion) => assertion.on_empty(),
            Self::OrderedVersion(assertion) => assertion.on_empty(),
            Self::List(_) | Self::InheritsWorkspace(_) => OnEmpty::Writes,
        }
    }
}

impl OnEmptyClass for ResolvedPackageFieldAssertion {
    fn on_empty(&self) -> OnEmpty {
        match self {
            Self::Scalar(assertion) => assertion.on_empty(),
            Self::OrderedVersion(assertion) => assertion.on_empty(),
            Self::List(_) | Self::InheritsWorkspace(_) => OnEmpty::Writes,
        }
    }
}

fn resolve_absent(
    key: &str,
    items: &[(Provenance, PackageFieldAssertion)],
    conflicts: &mut Vec<ConflictEntry>,
) -> Option<Option<ResolvedPackageFieldAssertion>> {
    let has_absent = items.iter().any(|(_, assertion)| {
        matches!(
            assertion,
            PackageFieldAssertion::Scalar(ScalarAssertion::Absent(_))
                | PackageFieldAssertion::OrderedVersion(ScalarAssertion::Absent(_))
        )
    });
    if !has_absent {
        return None;
    }
    if items.iter().all(|(_, assertion)| {
        matches!(
            assertion,
            PackageFieldAssertion::Scalar(ScalarAssertion::Absent(_))
                | PackageFieldAssertion::OrderedVersion(ScalarAssertion::Absent(_))
        )
    }) {
        return Some(Some(ResolvedPackageFieldAssertion::Scalar(
            ScalarAssertion::Absent(first_msg(items)),
        )));
    }
    push_conflict(key, items, conflicts);
    Some(None)
}

fn resolve_list_product(
    key: &str,
    items: &[(Provenance, PackageFieldAssertion)],
    conflicts: &mut Vec<ConflictEntry>,
) -> Option<Option<ResolvedPackageFieldAssertion>> {
    if !items
        .iter()
        .any(|(_, assertion)| matches!(assertion, PackageFieldAssertion::List(_)))
    {
        return None;
    }
    if !items.iter().all(|(_, assertion)| {
        matches!(assertion, PackageFieldAssertion::List(_)) || is_present(assertion)
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
    Some(Some(ResolvedPackageFieldAssertion::List(resolve_list(
        key, list_items, conflicts,
    ))))
}

fn resolve_inheritance(
    key: &str,
    items: &[(Provenance, PackageFieldAssertion)],
    conflicts: &mut Vec<ConflictEntry>,
) -> Option<Option<ResolvedPackageFieldAssertion>> {
    if !items
        .iter()
        .any(|(_, assertion)| matches!(assertion, PackageFieldAssertion::InheritsWorkspace(_)))
    {
        return None;
    }
    if !items.iter().all(|(_, assertion)| {
        matches!(assertion, PackageFieldAssertion::InheritsWorkspace(_)) || is_present(assertion)
    }) {
        push_conflict(key, items, conflicts);
        return Some(None);
    }
    Some(Some(ResolvedPackageFieldAssertion::InheritsWorkspace(
        items
            .iter()
            .find_map(|(_, assertion)| match assertion {
                PackageFieldAssertion::InheritsWorkspace(msg) => Some(msg.clone()),
                _ => None,
            })
            .unwrap_or_default(),
    )))
}

fn resolve_ordered_version(
    key: &str,
    items: &[(Provenance, PackageFieldAssertion)],
    conflicts: &mut Vec<ConflictEntry>,
) -> Option<Option<ResolvedPackageFieldAssertion>> {
    if !items
        .iter()
        .any(|(_, assertion)| matches!(assertion, PackageFieldAssertion::OrderedVersion(_)))
    {
        return None;
    }
    if !items.iter().all(|(_, assertion)| {
        matches!(assertion, PackageFieldAssertion::OrderedVersion(_)) || is_present(assertion)
    }) {
        push_conflict(key, items, conflicts);
        return Some(None);
    }
    let scalar_items = items
        .iter()
        .filter_map(|(prov, assertion)| match assertion {
            PackageFieldAssertion::OrderedVersion(assertion) => {
                Some((prov.clone(), assertion.clone()))
            }
            PackageFieldAssertion::Scalar(ScalarAssertion::Present(msg)) => {
                Some((prov.clone(), ScalarAssertion::Present(msg.clone())))
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    let resolved = ScalarAssertion::<DottedVersion>::resolve(key, scalar_items, conflicts)?;
    if matches!(
        resolved.merged,
        ScalarAssertion::AtMost(..) | ScalarAssertion::Range(..)
    ) {
        push_conflict(key, items, conflicts);
        return Some(None);
    }
    Some(Some(ResolvedPackageFieldAssertion::OrderedVersion(
        resolved.merged,
    )))
}

fn resolve_config_scalar(
    key: &str,
    items: Vec<(Provenance, PackageFieldAssertion)>,
    conflicts: &mut Vec<ConflictEntry>,
) -> Option<ResolvedRequirement<ResolvedPackageFieldAssertion, PackageFieldAssertion>> {
    if !items
        .iter()
        .all(|(_, assertion)| matches!(assertion, PackageFieldAssertion::Scalar(_)))
    {
        push_conflict(key, &items, conflicts);
        return None;
    }
    let scalar_items = items
        .iter()
        .filter_map(|(prov, assertion)| match assertion {
            PackageFieldAssertion::Scalar(assertion) => Some((prov.clone(), assertion.clone())),
            _ => None,
        })
        .collect::<Vec<_>>();
    let resolved = ScalarAssertion::<ConfigScalar>::resolve(key, scalar_items, conflicts)?;
    Some(ResolvedRequirement {
        merged: ResolvedPackageFieldAssertion::Scalar(resolved.merged),
        collected: items,
    })
}

fn is_present(assertion: &PackageFieldAssertion) -> bool {
    matches!(
        assertion,
        PackageFieldAssertion::Scalar(ScalarAssertion::Present(_))
            | PackageFieldAssertion::OrderedVersion(ScalarAssertion::Present(_))
    )
}

fn package_field_assertion_is_legal(key: &str, assertion: &PackageFieldAssertion) -> bool {
    let field = key.rsplit('.').next().unwrap_or(key);
    if key.starts_with("[workspace.package].")
        && matches!(assertion, PackageFieldAssertion::InheritsWorkspace(_))
    {
        return false;
    }
    if matches!(assertion, PackageFieldAssertion::InheritsWorkspace(_)) {
        return key.starts_with("[package].");
    }
    match field {
        "rust-version" | "version" => matches!(
            assertion,
            PackageFieldAssertion::OrderedVersion(ScalarAssertion::Equals(..))
                | PackageFieldAssertion::OrderedVersion(ScalarAssertion::AtLeast(..))
                | PackageFieldAssertion::OrderedVersion(ScalarAssertion::Present(_))
                | PackageFieldAssertion::OrderedVersion(ScalarAssertion::Absent(_))
                | PackageFieldAssertion::Scalar(ScalarAssertion::Present(_))
                | PackageFieldAssertion::Scalar(ScalarAssertion::Absent(_))
        ),
        "edition" => {
            matches!(
                assertion,
                PackageFieldAssertion::OrderedVersion(_)
                    | PackageFieldAssertion::Scalar(ScalarAssertion::Equals(
                        ConfigScalar::Str(_),
                        _
                    ))
                    | PackageFieldAssertion::Scalar(ScalarAssertion::Present(_))
                    | PackageFieldAssertion::Scalar(ScalarAssertion::Absent(_))
            ) || matches!(
                assertion,
                PackageFieldAssertion::Scalar(ScalarAssertion::OneOf(values, _))
                    if values.iter().all(|value| matches!(value, ConfigScalar::Str(_)))
            )
        }
        "license" => scalar_string_assertion_is_legal(assertion),
        "keywords" | "categories" => matches!(
            assertion,
            PackageFieldAssertion::List(_)
                | PackageFieldAssertion::Scalar(ScalarAssertion::Present(_))
                | PackageFieldAssertion::Scalar(ScalarAssertion::Absent(_))
        ),
        _ => match assertion {
            PackageFieldAssertion::OrderedVersion(_) => false,
            PackageFieldAssertion::InheritsWorkspace(_) => false,
            PackageFieldAssertion::Scalar(_) | PackageFieldAssertion::List(_) => true,
        },
    }
}

fn scalar_string_assertion_is_legal(assertion: &PackageFieldAssertion) -> bool {
    match assertion {
        PackageFieldAssertion::Scalar(ScalarAssertion::Equals(ConfigScalar::Str(_), _))
        | PackageFieldAssertion::Scalar(ScalarAssertion::Present(_))
        | PackageFieldAssertion::Scalar(ScalarAssertion::Absent(_)) => true,
        PackageFieldAssertion::Scalar(ScalarAssertion::OneOf(values, _)) => values
            .iter()
            .all(|value| matches!(value, ConfigScalar::Str(_))),
        _ => false,
    }
}

fn first_msg(items: &[(Provenance, PackageFieldAssertion)]) -> String {
    items
        .iter()
        .map(|(_, assertion)| match assertion {
            PackageFieldAssertion::Scalar(assertion) => assertion.message().to_owned(),
            PackageFieldAssertion::OrderedVersion(assertion) => assertion.message().to_owned(),
            PackageFieldAssertion::List(_) => String::new(),
            PackageFieldAssertion::InheritsWorkspace(msg) => msg.clone(),
        })
        .find(|msg| !msg.is_empty())
        .unwrap_or_default()
}

fn push_conflict(
    key: &str,
    items: &[(Provenance, PackageFieldAssertion)],
    conflicts: &mut Vec<ConflictEntry>,
) {
    push_debug_conflict(key, "scalar-disagree", items, conflicts);
}

fn push_unsupported_conflict(
    key: &str,
    items: &[(Provenance, PackageFieldAssertion)],
    conflicts: &mut Vec<ConflictEntry>,
) {
    push_debug_conflict(key, "scalar-operation-unsupported", items, conflicts);
}
