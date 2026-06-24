//! Target-table assertions: `[lib]` fields and the named `[[bin]]` /
//! `[[example]]` / `[[test]]` / `[[bench]]` entries.

#![cfg_attr(
    not(test),
    expect(
        clippy::missing_docs_in_private_items,
        reason = "Private target-table composition helpers are internal requirement steps."
    )
)]
#![expect(
    clippy::option_option,
    clippy::type_complexity,
    reason = "Target-table composition uses three-state resolution and core scalar assertions."
)]

use std::collections::BTreeMap;

use aqc_file_engine_core::{
    ConfigScalar, ConflictEntry, ListRequirements, OnEmpty, OnEmptyClass, Provenance, Resolve,
    ResolvedListRequirements, ResolvedRequirement, ScalarAssertion, resolve_list, resolve_map,
};

use super::helpers::push_debug_conflict;

/// What must hold about a single target-table field (`path`, `harness`,
/// `doctest`, `crate-type`, `required-features`, ...).
///
/// Equality (and therefore merge agreement) ignores the policy message.
#[derive(Debug, Clone)]
pub enum TargetFieldAssertion {
    Scalar(ScalarAssertion<ConfigScalar>),
    List(ListRequirements),
}

/// Resolved target-table field assertion.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResolvedTargetFieldAssertion {
    Scalar(ScalarAssertion<ConfigScalar>),
    List(ResolvedListRequirements),
}

/// Semantic equality: messages excluded.
impl PartialEq for TargetFieldAssertion {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Scalar(a), Self::Scalar(b)) => a == b,
            (Self::List(a), Self::List(b)) => a == b,
            (Self::Scalar(_), Self::List(_)) | (Self::List(_), Self::Scalar(_)) => false,
        }
    }
}

impl Resolve for TargetFieldAssertion {
    type Merged = ResolvedTargetFieldAssertion;

    fn resolve(
        key: &str,
        items: Vec<(Provenance, Self)>,
        conflicts: &mut Vec<ConflictEntry>,
    ) -> Option<ResolvedRequirement<Self::Merged, Self>> {
        if !items
            .iter()
            .all(|(_, assertion)| target_field_assertion_is_legal(key, assertion))
        {
            push_unsupported_field_conflict(key, &items, conflicts);
            return None;
        }
        if let Some(resolved) = resolve_list_or_present(key, &items, conflicts) {
            return resolved.map(|merged| ResolvedRequirement {
                merged,
                collected: items,
            });
        }
        compose_field_scalar(key, items, conflicts)
    }
}

impl OnEmptyClass for TargetFieldAssertion {
    fn on_empty(&self) -> OnEmpty {
        match self {
            Self::Scalar(assertion) => assertion.on_empty(),
            Self::List(_) => OnEmpty::Writes,
        }
    }
}

impl OnEmptyClass for ResolvedTargetFieldAssertion {
    fn on_empty(&self) -> OnEmpty {
        match self {
            Self::Scalar(assertion) => assertion.on_empty(),
            Self::List(_) => OnEmpty::Writes,
        }
    }
}

/// What must hold about one named array-of-tables target (`[[bin]]` etc.).
///
/// The map key supplies the required `name`, so `Present`/`Fields` are
/// writable; unasserted fields fall to cargo's auto-discovery defaults.
#[derive(Debug, Clone)]
pub enum TargetTableAssertion {
    /// A target with this name exists.
    Present(String),
    /// No target with this name exists.
    Absent(String),
    /// A target with this name exists and these fields hold.
    Fields(BTreeMap<String, TargetFieldAssertion>),
}

/// Resolved named target-table assertion.
#[derive(Debug, Clone, PartialEq)]
pub enum ResolvedTargetTableAssertion {
    /// A target with this name exists.
    Present(String),
    /// No target with this name exists.
    Absent(String),
    /// A target with this name exists and these fields hold.
    Fields(
        BTreeMap<String, ResolvedRequirement<ResolvedTargetFieldAssertion, TargetFieldAssertion>>,
    ),
}

/// Semantic equality: messages excluded.
impl PartialEq for TargetTableAssertion {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Present(_), Self::Present(_)) | (Self::Absent(_), Self::Absent(_)) => true,
            (Self::Fields(a), Self::Fields(b)) => a == b,
            (Self::Present(_) | Self::Fields(_), Self::Absent(_))
            | (Self::Present(_) | Self::Absent(_), Self::Fields(_))
            | (Self::Absent(_) | Self::Fields(_), Self::Present(_)) => false,
        }
    }
}

impl Resolve for TargetTableAssertion {
    type Merged = ResolvedTargetTableAssertion;

    fn resolve(
        key: &str,
        items: Vec<(Provenance, Self)>,
        conflicts: &mut Vec<ConflictEntry>,
    ) -> Option<ResolvedRequirement<Self::Merged, Self>> {
        if items.iter().any(|(_, a)| matches!(a, Self::Absent(_))) {
            if items.iter().all(|(_, a)| matches!(a, Self::Absent(_))) {
                return Some(ResolvedRequirement {
                    merged: ResolvedTargetTableAssertion::Absent(first_table_msg(&items)),
                    collected: items,
                });
            }
            push_table_conflict(key, &items, conflicts);
            return None;
        }
        if items.iter().any(|(_, a)| matches!(a, Self::Fields(_))) {
            if !items
                .iter()
                .all(|(_, a)| matches!(a, Self::Fields(_) | Self::Present(_)))
            {
                push_table_conflict(key, &items, conflicts);
                return None;
            }
            let maps = items
                .iter()
                .filter_map(|(p, a)| match a {
                    Self::Fields(m) => Some((p.clone(), m.clone())),
                    Self::Present(_) | Self::Absent(_) => None,
                })
                .collect();
            let fields = resolve_map(maps, |field| format!("{key}.{field}"), conflicts);
            return Some(ResolvedRequirement {
                merged: ResolvedTargetTableAssertion::Fields(fields),
                collected: items,
            });
        }
        Some(ResolvedRequirement {
            merged: ResolvedTargetTableAssertion::Present(first_table_msg(&items)),
            collected: items,
        })
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct TargetRequirements {
    pub lib_fields: BTreeMap<String, TargetFieldAssertion>,
    pub bin_targets: BTreeMap<String, TargetTableAssertion>,
    pub example_targets: BTreeMap<String, TargetTableAssertion>,
    pub test_targets: BTreeMap<String, TargetTableAssertion>,
    pub bench_targets: BTreeMap<String, TargetTableAssertion>,
}

#[derive(Debug, Clone, Default)]
pub struct ResolvedTargetRequirements {
    pub lib_fields:
        BTreeMap<String, ResolvedRequirement<ResolvedTargetFieldAssertion, TargetFieldAssertion>>,
    pub bin_targets:
        BTreeMap<String, ResolvedRequirement<ResolvedTargetTableAssertion, TargetTableAssertion>>,
    pub example_targets:
        BTreeMap<String, ResolvedRequirement<ResolvedTargetTableAssertion, TargetTableAssertion>>,
    pub test_targets:
        BTreeMap<String, ResolvedRequirement<ResolvedTargetTableAssertion, TargetTableAssertion>>,
    pub bench_targets:
        BTreeMap<String, ResolvedRequirement<ResolvedTargetTableAssertion, TargetTableAssertion>>,
}

impl TargetRequirements {
    pub fn resolve(
        items: Vec<(Provenance, Self)>,
        conflicts: &mut Vec<ConflictEntry>,
    ) -> ResolvedTargetRequirements {
        let mut lib_fields = Vec::new();
        let mut bin_targets = Vec::new();
        let mut example_targets = Vec::new();
        let mut test_targets = Vec::new();
        let mut bench_targets = Vec::new();
        for (prov, target) in items {
            lib_fields.push((prov.clone(), target.lib_fields));
            bin_targets.push((prov.clone(), target.bin_targets));
            example_targets.push((prov.clone(), target.example_targets));
            test_targets.push((prov.clone(), target.test_targets));
            bench_targets.push((prov, target.bench_targets));
        }
        ResolvedTargetRequirements {
            lib_fields: resolve_map(lib_fields, |f| format!("[lib].{f}"), conflicts),
            bin_targets: resolve_map(bin_targets, |n| format!("[[bin]].{n}"), conflicts),
            example_targets: resolve_map(
                example_targets,
                |n| format!("[[example]].{n}"),
                conflicts,
            ),
            test_targets: resolve_map(test_targets, |n| format!("[[test]].{n}"), conflicts),
            bench_targets: resolve_map(bench_targets, |n| format!("[[bench]].{n}"), conflicts),
        }
    }
}

impl OnEmptyClass for TargetTableAssertion {
    fn on_empty(&self) -> OnEmpty {
        match self {
            Self::Present(_) | Self::Absent(_) => OnEmpty::Writes,
            // Writable when every asserted field is writable.
            Self::Fields(map) => {
                if map.values().any(|a| a.on_empty() == OnEmpty::ChecksOnly) {
                    OnEmpty::ChecksOnly
                } else {
                    OnEmpty::Writes
                }
            }
        }
    }
}

impl OnEmptyClass for ResolvedTargetTableAssertion {
    fn on_empty(&self) -> OnEmpty {
        match self {
            Self::Present(_) | Self::Absent(_) => OnEmpty::Writes,
            Self::Fields(map) => {
                if map
                    .values()
                    .any(|resolved| resolved.merged.on_empty() == OnEmpty::ChecksOnly)
                {
                    OnEmpty::ChecksOnly
                } else {
                    OnEmpty::Writes
                }
            }
        }
    }
}

fn resolve_list_or_present(
    key: &str,
    items: &[(Provenance, TargetFieldAssertion)],
    conflicts: &mut Vec<ConflictEntry>,
) -> Option<Option<ResolvedTargetFieldAssertion>> {
    let has_list = items
        .iter()
        .any(|(_, a)| matches!(a, TargetFieldAssertion::List(_)));
    if !has_list {
        return None;
    }
    if !items.iter().all(|(_, a)| {
        matches!(
            a,
            TargetFieldAssertion::List(_)
                | TargetFieldAssertion::Scalar(ScalarAssertion::Present(_))
        )
    }) {
        push_field_conflict(key, items, conflicts);
        return Some(None);
    }
    let list_items = items
        .iter()
        .filter_map(|(prov, assertion)| match assertion {
            TargetFieldAssertion::List(list) => Some((prov.clone(), list.clone())),
            TargetFieldAssertion::Scalar(_) => None,
        })
        .collect();
    Some(Some(ResolvedTargetFieldAssertion::List(resolve_list(
        key, list_items, conflicts,
    ))))
}

fn compose_field_scalar(
    key: &str,
    items: Vec<(Provenance, TargetFieldAssertion)>,
    conflicts: &mut Vec<ConflictEntry>,
) -> Option<ResolvedRequirement<ResolvedTargetFieldAssertion, TargetFieldAssertion>> {
    if !items
        .iter()
        .all(|(_, assertion)| matches!(assertion, TargetFieldAssertion::Scalar(_)))
    {
        push_field_conflict(key, &items, conflicts);
        return None;
    }
    let scalar_items = items
        .iter()
        .filter_map(|(prov, assertion)| match assertion {
            TargetFieldAssertion::Scalar(assertion) => Some((prov.clone(), assertion.clone())),
            TargetFieldAssertion::List(_) => None,
        })
        .collect();
    let resolved = ScalarAssertion::<ConfigScalar>::resolve(key, scalar_items, conflicts)?;

    Some(ResolvedRequirement {
        merged: ResolvedTargetFieldAssertion::Scalar(resolved.merged),
        collected: items,
    })
}

fn first_table_msg(items: &[(Provenance, TargetTableAssertion)]) -> String {
    items
        .iter()
        .find_map(|(_, assertion)| match assertion {
            TargetTableAssertion::Present(msg) | TargetTableAssertion::Absent(msg) => {
                Some(msg.clone())
            }
            TargetTableAssertion::Fields(_) => None,
        })
        .unwrap_or_default()
}

fn push_field_conflict(
    key: &str,
    items: &[(Provenance, TargetFieldAssertion)],
    conflicts: &mut Vec<ConflictEntry>,
) {
    push_debug_conflict(key, "scalar-disagree", items, conflicts);
}

fn target_field_assertion_is_legal(key: &str, assertion: &TargetFieldAssertion) -> bool {
    let field = key.rsplit('.').next().unwrap_or(key);
    match field {
        "crate-type" | "required-features" => matches!(
            assertion,
            TargetFieldAssertion::List(_)
                | TargetFieldAssertion::Scalar(
                    ScalarAssertion::Present(_) | ScalarAssertion::Absent(_)
                )
        ),
        _ => matches!(assertion, TargetFieldAssertion::Scalar(_)),
    }
}

fn push_unsupported_field_conflict(
    key: &str,
    items: &[(Provenance, TargetFieldAssertion)],
    conflicts: &mut Vec<ConflictEntry>,
) {
    push_debug_conflict(key, "scalar-operation-unsupported", items, conflicts);
}

fn push_table_conflict(
    key: &str,
    items: &[(Provenance, TargetTableAssertion)],
    conflicts: &mut Vec<ConflictEntry>,
) {
    push_debug_conflict(key, "scalar-disagree", items, conflicts);
}
