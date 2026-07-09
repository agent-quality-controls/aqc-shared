//! `[package].<field>` / `[workspace.package].<field>` assertions.

#![cfg_attr(
    not(test),
    expect(
        clippy::missing_docs_in_private_items,
        reason = "Private package-field composition helpers are internal requirement steps."
    )
)]
#![allow(
    clippy::option_option,
    clippy::type_complexity,
    reason = "Package-field composition uses product wrappers and three-state resolution."
)]

use aqc_file_engine_core::{
    ConfigScalar, ConflictEntry, DottedVersion, ListRequirements, OnEmpty, OnEmptyClass,
    Provenance, Resolve, ResolvedListRequirements, ResolvedRequirement, ScalarAssertion,
    push_conflict as push_core_conflict, render_list_requirement, render_scalar_assertion,
    resolve_list,
};

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
            (
                Self::Scalar(_) | Self::List(_) | Self::InheritsWorkspace(_),
                Self::OrderedVersion(_),
            )
            | (
                Self::Scalar(_) | Self::OrderedVersion(_) | Self::InheritsWorkspace(_),
                Self::List(_),
            )
            | (
                Self::Scalar(_) | Self::OrderedVersion(_) | Self::List(_),
                Self::InheritsWorkspace(_),
            )
            | (
                Self::OrderedVersion(_) | Self::List(_) | Self::InheritsWorkspace(_),
                Self::Scalar(_),
            ) => false,
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
    push_scalar_disagree_conflict(key, items, conflicts);
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
        push_scalar_disagree_conflict(key, items, conflicts);
        return Some(None);
    }
    let list_items = items
        .iter()
        .filter_map(|(prov, assertion)| match assertion {
            PackageFieldAssertion::List(list) => Some((prov.clone(), list.clone())),
            PackageFieldAssertion::Scalar(_)
            | PackageFieldAssertion::OrderedVersion(_)
            | PackageFieldAssertion::InheritsWorkspace(_) => None,
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
        push_scalar_disagree_conflict(key, items, conflicts);
        return Some(None);
    }
    Some(Some(ResolvedPackageFieldAssertion::InheritsWorkspace(
        items
            .iter()
            .find_map(|(_, assertion)| match assertion {
                PackageFieldAssertion::InheritsWorkspace(msg) => Some(msg.clone()),
                PackageFieldAssertion::Scalar(_)
                | PackageFieldAssertion::OrderedVersion(_)
                | PackageFieldAssertion::List(_) => None,
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
        push_scalar_disagree_conflict(key, items, conflicts);
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
            PackageFieldAssertion::Scalar(_)
            | PackageFieldAssertion::List(_)
            | PackageFieldAssertion::InheritsWorkspace(_) => None,
        })
        .collect::<Vec<_>>();
    let resolved = ScalarAssertion::<DottedVersion>::resolve(key, scalar_items, conflicts)?;
    if matches!(
        resolved.merged,
        ScalarAssertion::AtMost(..) | ScalarAssertion::Range(..)
    ) {
        push_scalar_disagree_conflict(key, items, conflicts);
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
        push_scalar_disagree_conflict(key, &items, conflicts);
        return None;
    }
    let scalar_items = items
        .iter()
        .filter_map(|(prov, assertion)| match assertion {
            PackageFieldAssertion::Scalar(assertion) => Some((prov.clone(), assertion.clone())),
            PackageFieldAssertion::OrderedVersion(_)
            | PackageFieldAssertion::List(_)
            | PackageFieldAssertion::InheritsWorkspace(_) => None,
        })
        .collect::<Vec<_>>();
    let resolved = ScalarAssertion::<ConfigScalar>::resolve(key, scalar_items, conflicts)?;
    Some(ResolvedRequirement {
        merged: ResolvedPackageFieldAssertion::Scalar(resolved.merged),
        collected: items,
    })
}

const fn is_present(assertion: &PackageFieldAssertion) -> bool {
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
            PackageFieldAssertion::OrderedVersion(
                ScalarAssertion::Equals(..)
                    | ScalarAssertion::AtLeast(..)
                    | ScalarAssertion::Present(_)
                    | ScalarAssertion::Absent(_)
            ) | PackageFieldAssertion::Scalar(
                ScalarAssertion::Present(_) | ScalarAssertion::Absent(_)
            )
        ),
        "edition" => {
            matches!(
                assertion,
                PackageFieldAssertion::OrderedVersion(_)
                    | PackageFieldAssertion::Scalar(
                        ScalarAssertion::Equals(ConfigScalar::Str(_), _)
                            | ScalarAssertion::Present(_)
                            | ScalarAssertion::Absent(_)
                    )
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
                | PackageFieldAssertion::Scalar(
                    ScalarAssertion::Present(_) | ScalarAssertion::Absent(_)
                )
        ),
        _ => match assertion {
            PackageFieldAssertion::OrderedVersion(_)
            | PackageFieldAssertion::InheritsWorkspace(_) => false,
            PackageFieldAssertion::Scalar(_) | PackageFieldAssertion::List(_) => true,
        },
    }
}

fn scalar_string_assertion_is_legal(assertion: &PackageFieldAssertion) -> bool {
    match assertion {
        PackageFieldAssertion::Scalar(
            ScalarAssertion::Equals(ConfigScalar::Str(_), _)
            | ScalarAssertion::Present(_)
            | ScalarAssertion::Absent(_),
        ) => true,
        PackageFieldAssertion::Scalar(ScalarAssertion::OneOf(values, _)) => values
            .iter()
            .all(|value| matches!(value, ConfigScalar::Str(_))),
        PackageFieldAssertion::Scalar(_)
        | PackageFieldAssertion::OrderedVersion(_)
        | PackageFieldAssertion::List(_)
        | PackageFieldAssertion::InheritsWorkspace(_) => false,
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

fn push_scalar_disagree_conflict(
    key: &str,
    items: &[(Provenance, PackageFieldAssertion)],
    conflicts: &mut Vec<ConflictEntry>,
) {
    push_core_conflict(
        key,
        "scalar-disagree",
        items,
        render_package_field_assertion,
        conflicts,
    );
}

fn push_unsupported_conflict(
    key: &str,
    items: &[(Provenance, PackageFieldAssertion)],
    conflicts: &mut Vec<ConflictEntry>,
) {
    push_core_conflict(
        key,
        "scalar-operation-unsupported",
        items,
        render_package_field_assertion,
        conflicts,
    );
}

fn render_package_field_assertion(assertion: &PackageFieldAssertion) -> String {
    match assertion {
        PackageFieldAssertion::Scalar(assertion) => render_scalar_assertion(assertion),
        PackageFieldAssertion::OrderedVersion(assertion) => render_scalar_assertion(assertion),
        PackageFieldAssertion::List(requirements) => render_list_requirement(requirements),
        PackageFieldAssertion::InheritsWorkspace(_) => "inherits workspace".to_owned(),
    }
}
