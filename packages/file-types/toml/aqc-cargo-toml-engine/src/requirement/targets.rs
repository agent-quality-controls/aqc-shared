//! Target-table assertions: `[lib]` fields and the named `[[bin]]` /
//! `[[example]]` / `[[test]]` / `[[bench]]` entries.

use std::collections::{BTreeMap, BTreeSet};

use aqc_file_engine_core::{
    ConfigScalar, ConflictEntry, ListRequirements, OnEmpty, OnEmptyClass, Provenance, Resolve,
    ResolvedListRequirements, ResolvedRequirement, resolve_list, resolve_map,
};

/// What must hold about a single target-table field (`path`, `harness`,
/// `doctest`, `crate-type`, `required-features`, ...).
///
/// Equality (and therefore merge agreement) ignores the policy message.
#[derive(Debug, Clone)]
pub enum TargetFieldAssertion {
    /// The field equals this value.
    Equals(ConfigScalar, String),
    /// The field's value is one of these (check-only).
    OneOf(BTreeSet<String>, String),
    /// List product requirements for this field.
    List(ListRequirements),
    /// The field is set, to anything (check-only).
    Present(String),
    /// The field is not set.
    Absent(String),
}

/// Resolved target-table field assertion.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResolvedTargetFieldAssertion {
    /// The field equals this value.
    Equals(ConfigScalar, String),
    /// The field's value is one of these.
    OneOf(BTreeSet<String>, String),
    /// Resolved list product requirements for this field.
    List(ResolvedListRequirements),
    /// The field is set, to anything.
    Present(String),
    /// The field is not set.
    Absent(String),
}

/// Semantic equality: messages excluded.
impl PartialEq for TargetFieldAssertion {
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

impl Resolve for TargetFieldAssertion {
    type Merged = ResolvedTargetFieldAssertion;

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
                    merged: ResolvedTargetFieldAssertion::Absent(first_field_msg(&items)),
                    collected: items,
                });
            }
            push_field_conflict(key, &items, conflicts);
            return None;
        }
        compose_field_scalar(key, items, conflicts)
    }
}

impl OnEmptyClass for TargetFieldAssertion {
    fn on_empty(&self) -> OnEmpty {
        match self {
            Self::Equals(..) | Self::List(..) | Self::Absent(..) => OnEmpty::Writes,
            Self::OneOf(..) | Self::Present(..) => OnEmpty::ChecksOnly,
        }
    }
}

impl OnEmptyClass for ResolvedTargetFieldAssertion {
    fn on_empty(&self) -> OnEmpty {
        match self {
            Self::Equals(..) | Self::List(..) | Self::Absent(..) => OnEmpty::Writes,
            Self::OneOf(..) | Self::Present(..) => OnEmpty::ChecksOnly,
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
            _ => false,
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
            TargetFieldAssertion::List(_) | TargetFieldAssertion::Present(_)
        )
    }) {
        push_field_conflict(key, items, conflicts);
        return Some(None);
    }
    let list_items = items
        .iter()
        .filter_map(|(prov, assertion)| match assertion {
            TargetFieldAssertion::List(list) => Some((prov.clone(), list.clone())),
            _ => None,
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
    let equals = items
        .iter()
        .filter_map(|(_, assertion)| match assertion {
            TargetFieldAssertion::Equals(value, msg) => Some((value.clone(), msg.clone())),
            _ => None,
        })
        .collect::<Vec<_>>();
    let oneof = intersect_oneofs(
        items
            .iter()
            .filter_map(|(_, assertion)| match assertion {
                TargetFieldAssertion::OneOf(values, msg) => Some((values.clone(), msg.clone())),
                _ => None,
            })
            .collect(),
    );

    let merged = if equals.windows(2).any(|pair| pair[0].0 != pair[1].0) {
        push_field_conflict(key, &items, conflicts);
        return None;
    } else if let Some((value, msg)) = equals.first() {
        if oneof
            .as_ref()
            .is_some_and(|(allowed, _)| !allowed.contains(&scalar_text(value)))
        {
            push_field_conflict(key, &items, conflicts);
            return None;
        }
        ResolvedTargetFieldAssertion::Equals(value.clone(), msg.clone())
    } else if let Some((allowed, msg)) = oneof {
        if allowed.is_empty() {
            push_field_conflict(key, &items, conflicts);
            return None;
        }
        ResolvedTargetFieldAssertion::OneOf(allowed, msg)
    } else {
        ResolvedTargetFieldAssertion::Present(first_field_msg(&items))
    };

    Some(ResolvedRequirement {
        merged,
        collected: items,
    })
}

fn intersect_oneofs(oneofs: Vec<(BTreeSet<String>, String)>) -> Option<(BTreeSet<String>, String)> {
    let mut iter = oneofs.into_iter();
    let (mut out, msg) = iter.next()?;
    for (next, _) in iter {
        out = out.intersection(&next).cloned().collect();
    }
    Some((out, msg))
}

fn first_field_msg(items: &[(Provenance, TargetFieldAssertion)]) -> String {
    items
        .iter()
        .find_map(|(_, assertion)| match assertion {
            TargetFieldAssertion::Equals(_, msg)
            | TargetFieldAssertion::OneOf(_, msg)
            | TargetFieldAssertion::Present(msg)
            | TargetFieldAssertion::Absent(msg) => Some(msg.clone()),
            TargetFieldAssertion::List(_) => None,
        })
        .unwrap_or_default()
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

fn scalar_text(value: &ConfigScalar) -> String {
    match value {
        ConfigScalar::Str(value) => value.clone(),
        ConfigScalar::Int(value) => value.to_string(),
        ConfigScalar::Bool(value) => value.to_string(),
    }
}

fn push_field_conflict(
    key: &str,
    items: &[(Provenance, TargetFieldAssertion)],
    conflicts: &mut Vec<ConflictEntry>,
) {
    conflicts.push(ConflictEntry {
        key: key.to_owned(),
        reason: "scalar-disagree".to_owned(),
        contributors: items
            .iter()
            .map(|(prov, value)| (prov.clone(), format!("{value:?}")))
            .collect(),
    });
}

fn push_table_conflict(
    key: &str,
    items: &[(Provenance, TargetTableAssertion)],
    conflicts: &mut Vec<ConflictEntry>,
) {
    conflicts.push(ConflictEntry {
        key: key.to_owned(),
        reason: "scalar-disagree".to_owned(),
        contributors: items
            .iter()
            .map(|(prov, value)| (prov.clone(), format!("{value:?}")))
            .collect(),
    });
}
