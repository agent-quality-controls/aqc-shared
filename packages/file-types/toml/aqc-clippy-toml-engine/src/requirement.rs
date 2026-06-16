//! Declarative requirement and assertion types accepted by `ClippyTomlEngine`.

#![expect(
    clippy::disallowed_types,
    reason = "`Any` is used only for EngineRequirement downcast dispatch."
)]

use core::any::Any;
use std::collections::{BTreeMap, BTreeSet};

use aqc_file_engine_core::{
    ConflictEntry, EngineRequirement, FileItemRequirement, ItemRequirements, Provenance, Resolve,
    ResolvedItemRequirements, ResolvedRequirement, compose_item_by, parse_version_tuple,
    resolve_items, resolve_map, resolve_maybe, strongest_version_floor,
};

#[derive(Debug, Clone, Default)]
pub struct ClippyTomlRequirements {
    pub msrv: Option<MsrvAssertion>,
    pub thresholds: BTreeMap<String, NumericAssertion>,
    pub disallowed_methods: ItemRequirements<BanEntry>,
    pub disallowed_types: ItemRequirements<BanEntry>,
    pub disallowed_macros: ItemRequirements<BanEntry>,
    pub bools: BTreeMap<String, BoolAssertion>,
    pub enums: BTreeMap<String, StringAssertion>,
}

#[derive(Debug, Clone, Default)]
pub struct ResolvedClippyTomlRequirements {
    pub msrv: Option<ResolvedRequirement<MsrvAssertion, MsrvAssertion>>,
    pub thresholds: BTreeMap<String, ResolvedRequirement<NumericAssertion, NumericAssertion>>,
    pub disallowed_methods: ResolvedItemRequirements<BanEntry>,
    pub disallowed_types: ResolvedItemRequirements<BanEntry>,
    pub disallowed_macros: ResolvedItemRequirements<BanEntry>,
    pub bools: BTreeMap<String, ResolvedRequirement<BoolAssertion, BoolAssertion>>,
    pub enums: BTreeMap<String, ResolvedRequirement<StringAssertion, StringAssertion>>,
}

impl ClippyTomlRequirements {
    #[must_use]
    pub fn merge(
        reqs: Vec<(Provenance, ClippyTomlRequirements)>,
    ) -> (ResolvedClippyTomlRequirements, Vec<ConflictEntry>) {
        let mut conflicts = Vec::new();
        let out = ResolvedClippyTomlRequirements {
            msrv: resolve_maybe(
                "msrv",
                reqs.iter()
                    .map(|(prov, req)| (prov.clone(), req.msrv.clone()))
                    .collect(),
                &mut conflicts,
            ),
            thresholds: resolve_map(
                reqs.iter()
                    .map(|(prov, req)| (prov.clone(), req.thresholds.clone()))
                    .collect(),
                Clone::clone,
                &mut conflicts,
            ),
            disallowed_methods: resolve_items(
                "disallowed-methods",
                reqs.iter()
                    .map(|(prov, req)| (prov.clone(), req.disallowed_methods.clone()))
                    .collect(),
                &mut conflicts,
            ),
            disallowed_types: resolve_items(
                "disallowed-types",
                reqs.iter()
                    .map(|(prov, req)| (prov.clone(), req.disallowed_types.clone()))
                    .collect(),
                &mut conflicts,
            ),
            disallowed_macros: resolve_items(
                "disallowed-macros",
                reqs.iter()
                    .map(|(prov, req)| (prov.clone(), req.disallowed_macros.clone()))
                    .collect(),
                &mut conflicts,
            ),
            bools: resolve_map(
                reqs.iter()
                    .map(|(prov, req)| (prov.clone(), req.bools.clone()))
                    .collect(),
                Clone::clone,
                &mut conflicts,
            ),
            enums: resolve_map(
                reqs.iter()
                    .map(|(prov, req)| (prov.clone(), req.enums.clone()))
                    .collect(),
                Clone::clone,
                &mut conflicts,
            ),
        };
        (out, conflicts)
    }
}

#[derive(Debug, Clone)]
pub enum MsrvAssertion {
    Equals(String, String),
    AtLeast(String, String),
    OneOf(BTreeSet<String>, String),
    Present(String),
    Absent(String),
}

impl PartialEq for MsrvAssertion {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Equals(a, _), Self::Equals(b, _))
            | (Self::AtLeast(a, _), Self::AtLeast(b, _)) => a == b,
            (Self::OneOf(a, _), Self::OneOf(b, _)) => a == b,
            (Self::Present(_), Self::Present(_)) | (Self::Absent(_), Self::Absent(_)) => true,
            _ => false,
        }
    }
}

impl Resolve for MsrvAssertion {
    type Merged = Self;

    fn resolve(
        key: &str,
        items: Vec<(Provenance, Self)>,
        conflicts: &mut Vec<ConflictEntry>,
    ) -> Option<ResolvedRequirement<Self::Merged, Self>> {
        if items
            .iter()
            .any(|(_, item)| matches!(item, Self::Absent(_)))
        {
            if items
                .iter()
                .all(|(_, item)| matches!(item, Self::Absent(_)))
            {
                return Some(ResolvedRequirement {
                    merged: Self::Absent(msrv_msg(&items)),
                    collected: items,
                });
            }
            push_msrv_conflict(key, &items, conflicts);
            return None;
        }

        let equals = items
            .iter()
            .filter_map(|(_, item)| match item {
                Self::Equals(version, msg) => Some((version.clone(), msg.clone())),
                _ => None,
            })
            .collect::<Vec<_>>();
        let floors = items
            .iter()
            .filter_map(|(_, item)| match item {
                Self::AtLeast(version, msg) => Some((version.clone(), msg.clone())),
                _ => None,
            })
            .collect::<Vec<_>>();
        let oneof = intersect_string_sets(
            items
                .iter()
                .filter_map(|(_, item)| match item {
                    Self::OneOf(values, msg) => Some((values.clone(), msg.clone())),
                    _ => None,
                })
                .collect(),
        );

        let floor = if floors.is_empty() {
            None
        } else {
            Some(strongest_version_floor(floors))
        };
        let merged = if equals.windows(2).any(|pair| pair[0].0 != pair[1].0) {
            push_msrv_conflict(key, &items, conflicts);
            return None;
        } else if let Some((version, msg)) = equals.first() {
            if oneof
                .as_ref()
                .is_some_and(|(allowed, _)| !allowed.contains(version))
                || floor
                    .as_ref()
                    .is_some_and(|(min, _)| parse_version_tuple(version) < parse_version_tuple(min))
            {
                push_msrv_conflict(key, &items, conflicts);
                return None;
            }
            Self::Equals(version.clone(), msg.clone())
        } else if let Some((min, min_msg)) = floor {
            if let Some((allowed, allowed_msg)) = oneof {
                let filtered = allowed
                    .into_iter()
                    .filter(|value| parse_version_tuple(value) >= parse_version_tuple(&min))
                    .collect::<BTreeSet<_>>();
                if filtered.is_empty() {
                    push_msrv_conflict(key, &items, conflicts);
                    return None;
                }
                Self::OneOf(filtered, allowed_msg)
            } else {
                Self::AtLeast(min, min_msg)
            }
        } else if let Some((allowed, msg)) = oneof {
            if allowed.is_empty() {
                push_msrv_conflict(key, &items, conflicts);
                return None;
            }
            Self::OneOf(allowed, msg)
        } else {
            Self::Present(msrv_msg(&items))
        };

        Some(ResolvedRequirement {
            merged,
            collected: items,
        })
    }
}

#[derive(Debug, Clone)]
pub enum NumericAssertion {
    Equals(u64, String),
    AtMost(u64, String),
    AtLeast(u64, String),
    Range(u64, u64, String),
    Present(String),
    Absent(String),
}

impl PartialEq for NumericAssertion {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Equals(a, _), Self::Equals(b, _))
            | (Self::AtMost(a, _), Self::AtMost(b, _))
            | (Self::AtLeast(a, _), Self::AtLeast(b, _)) => a == b,
            (Self::Range(a_min, a_max, _), Self::Range(b_min, b_max, _)) => {
                a_min == b_min && a_max == b_max
            }
            (Self::Present(_), Self::Present(_)) | (Self::Absent(_), Self::Absent(_)) => true,
            _ => false,
        }
    }
}

impl Resolve for NumericAssertion {
    type Merged = Self;

    fn resolve(
        key: &str,
        items: Vec<(Provenance, Self)>,
        conflicts: &mut Vec<ConflictEntry>,
    ) -> Option<ResolvedRequirement<Self::Merged, Self>> {
        if items
            .iter()
            .any(|(_, item)| matches!(item, Self::Absent(_)))
        {
            if items
                .iter()
                .all(|(_, item)| matches!(item, Self::Absent(_)))
            {
                return Some(ResolvedRequirement {
                    merged: Self::Absent(numeric_msg(&items)),
                    collected: items,
                });
            }
            push_numeric_conflict(key, &items, conflicts);
            return None;
        }

        let equals = items
            .iter()
            .filter_map(|(_, item)| match item {
                Self::Equals(value, msg) => Some((*value, msg.clone())),
                _ => None,
            })
            .collect::<Vec<_>>();
        let min = strongest_numeric_floor(&items);
        let max = strongest_numeric_ceiling(&items);

        let merged = if equals.windows(2).any(|pair| pair[0].0 != pair[1].0) {
            push_numeric_conflict(key, &items, conflicts);
            return None;
        } else if let Some((value, msg)) = equals.first() {
            if min.as_ref().is_some_and(|(floor, _)| value < floor)
                || max.as_ref().is_some_and(|(ceiling, _)| value > ceiling)
            {
                push_numeric_conflict(key, &items, conflicts);
                return None;
            }
            Self::Equals(*value, msg.clone())
        } else {
            match (min, max) {
                (Some((floor, floor_msg)), Some((ceiling, ceiling_msg))) => {
                    if floor > ceiling {
                        push_numeric_conflict(key, &items, conflicts);
                        return None;
                    }
                    if floor == ceiling {
                        Self::Equals(floor, floor_msg)
                    } else {
                        Self::Range(floor, ceiling, format!("{floor_msg}; {ceiling_msg}"))
                    }
                }
                (Some((floor, msg)), None) => Self::AtLeast(floor, msg),
                (None, Some((ceiling, msg))) => Self::AtMost(ceiling, msg),
                (None, None) => Self::Present(numeric_msg(&items)),
            }
        };

        Some(ResolvedRequirement {
            merged,
            collected: items,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BanEntry {
    pub path: String,
    pub message: String,
}

impl FileItemRequirement for BanEntry {
    type Identity = String;

    fn merge_identity(&self) -> Self::Identity {
        self.path.clone()
    }

    fn compose_item(
        key: &str,
        items: Vec<(Provenance, (Self, String))>,
        conflicts: &mut Vec<ConflictEntry>,
    ) -> Option<ResolvedRequirement<Self, (Self, String)>> {
        compose_item_by(key, items, |entry| entry.path.clone(), conflicts)
    }
}

#[derive(Debug, Clone)]
pub enum BoolAssertion {
    Equals(bool, String),
    Present(String),
    Absent(String),
}

impl PartialEq for BoolAssertion {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Equals(a, _), Self::Equals(b, _)) => a == b,
            (Self::Present(_), Self::Present(_)) | (Self::Absent(_), Self::Absent(_)) => true,
            _ => false,
        }
    }
}

impl Resolve for BoolAssertion {
    type Merged = Self;

    fn resolve(
        key: &str,
        items: Vec<(Provenance, Self)>,
        conflicts: &mut Vec<ConflictEntry>,
    ) -> Option<ResolvedRequirement<Self::Merged, Self>> {
        if items
            .iter()
            .any(|(_, item)| matches!(item, Self::Absent(_)))
        {
            if items
                .iter()
                .all(|(_, item)| matches!(item, Self::Absent(_)))
            {
                return Some(ResolvedRequirement {
                    merged: Self::Absent(bool_msg(&items)),
                    collected: items,
                });
            }
            push_bool_conflict(key, &items, conflicts);
            return None;
        }
        let equals = items
            .iter()
            .filter_map(|(_, item)| match item {
                Self::Equals(value, msg) => Some((*value, msg.clone())),
                _ => None,
            })
            .collect::<Vec<_>>();
        if equals.windows(2).any(|pair| pair[0].0 != pair[1].0) {
            push_bool_conflict(key, &items, conflicts);
            return None;
        }
        let merged = equals.first().map_or_else(
            || Self::Present(bool_msg(&items)),
            |(value, msg)| Self::Equals(*value, msg.clone()),
        );
        Some(ResolvedRequirement {
            merged,
            collected: items,
        })
    }
}

#[derive(Debug, Clone)]
pub enum StringAssertion {
    Equals(String, String),
    OneOf(BTreeSet<String>, String),
    Present(String),
    Absent(String),
}

impl PartialEq for StringAssertion {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Equals(a, _), Self::Equals(b, _)) => a == b,
            (Self::OneOf(a, _), Self::OneOf(b, _)) => a == b,
            (Self::Present(_), Self::Present(_)) | (Self::Absent(_), Self::Absent(_)) => true,
            _ => false,
        }
    }
}

impl Resolve for StringAssertion {
    type Merged = Self;

    fn resolve(
        key: &str,
        items: Vec<(Provenance, Self)>,
        conflicts: &mut Vec<ConflictEntry>,
    ) -> Option<ResolvedRequirement<Self::Merged, Self>> {
        if items
            .iter()
            .any(|(_, item)| matches!(item, Self::Absent(_)))
        {
            if items
                .iter()
                .all(|(_, item)| matches!(item, Self::Absent(_)))
            {
                return Some(ResolvedRequirement {
                    merged: Self::Absent(string_msg(&items)),
                    collected: items,
                });
            }
            push_string_conflict(key, &items, conflicts);
            return None;
        }
        let equals = items
            .iter()
            .filter_map(|(_, item)| match item {
                Self::Equals(value, msg) => Some((value.clone(), msg.clone())),
                _ => None,
            })
            .collect::<Vec<_>>();
        let oneof = intersect_string_sets(
            items
                .iter()
                .filter_map(|(_, item)| match item {
                    Self::OneOf(values, msg) => Some((values.clone(), msg.clone())),
                    _ => None,
                })
                .collect(),
        );
        let merged = if equals.windows(2).any(|pair| pair[0].0 != pair[1].0) {
            push_string_conflict(key, &items, conflicts);
            return None;
        } else if let Some((value, msg)) = equals.first() {
            if oneof
                .as_ref()
                .is_some_and(|(allowed, _)| !allowed.contains(value))
            {
                push_string_conflict(key, &items, conflicts);
                return None;
            }
            Self::Equals(value.clone(), msg.clone())
        } else if let Some((allowed, msg)) = oneof {
            if allowed.is_empty() {
                push_string_conflict(key, &items, conflicts);
                return None;
            }
            Self::OneOf(allowed, msg)
        } else {
            Self::Present(string_msg(&items))
        };
        Some(ResolvedRequirement {
            merged,
            collected: items,
        })
    }
}

fn intersect_string_sets(
    oneofs: Vec<(BTreeSet<String>, String)>,
) -> Option<(BTreeSet<String>, String)> {
    let mut iter = oneofs.into_iter();
    let (mut out, msg) = iter.next()?;
    for (next, _) in iter {
        out = out.intersection(&next).cloned().collect();
    }
    Some((out, msg))
}

fn strongest_numeric_floor(items: &[(Provenance, NumericAssertion)]) -> Option<(u64, String)> {
    items
        .iter()
        .filter_map(|(_, item)| match item {
            NumericAssertion::AtLeast(value, msg) => Some((*value, msg.clone())),
            NumericAssertion::Range(min, _, msg) => Some((*min, msg.clone())),
            _ => None,
        })
        .max_by_key(|(value, _)| *value)
}

fn strongest_numeric_ceiling(items: &[(Provenance, NumericAssertion)]) -> Option<(u64, String)> {
    items
        .iter()
        .filter_map(|(_, item)| match item {
            NumericAssertion::AtMost(value, msg) => Some((*value, msg.clone())),
            NumericAssertion::Range(_, max, msg) => Some((*max, msg.clone())),
            _ => None,
        })
        .min_by_key(|(value, _)| *value)
}

fn msrv_msg(items: &[(Provenance, MsrvAssertion)]) -> String {
    items
        .iter()
        .find_map(|(_, item)| match item {
            MsrvAssertion::Equals(_, msg)
            | MsrvAssertion::AtLeast(_, msg)
            | MsrvAssertion::OneOf(_, msg)
            | MsrvAssertion::Present(msg)
            | MsrvAssertion::Absent(msg) => Some(msg.clone()),
        })
        .unwrap_or_default()
}

fn numeric_msg(items: &[(Provenance, NumericAssertion)]) -> String {
    items
        .iter()
        .find_map(|(_, item)| match item {
            NumericAssertion::Equals(_, msg)
            | NumericAssertion::AtMost(_, msg)
            | NumericAssertion::AtLeast(_, msg)
            | NumericAssertion::Range(_, _, msg)
            | NumericAssertion::Present(msg)
            | NumericAssertion::Absent(msg) => Some(msg.clone()),
        })
        .unwrap_or_default()
}

fn bool_msg(items: &[(Provenance, BoolAssertion)]) -> String {
    items
        .iter()
        .find_map(|(_, item)| match item {
            BoolAssertion::Equals(_, msg)
            | BoolAssertion::Present(msg)
            | BoolAssertion::Absent(msg) => Some(msg.clone()),
        })
        .unwrap_or_default()
}

fn string_msg(items: &[(Provenance, StringAssertion)]) -> String {
    items
        .iter()
        .find_map(|(_, item)| match item {
            StringAssertion::Equals(_, msg)
            | StringAssertion::OneOf(_, msg)
            | StringAssertion::Present(msg)
            | StringAssertion::Absent(msg) => Some(msg.clone()),
        })
        .unwrap_or_default()
}

fn push_msrv_conflict(
    key: &str,
    items: &[(Provenance, MsrvAssertion)],
    conflicts: &mut Vec<ConflictEntry>,
) {
    push_conflict(key, items, conflicts);
}

fn push_numeric_conflict(
    key: &str,
    items: &[(Provenance, NumericAssertion)],
    conflicts: &mut Vec<ConflictEntry>,
) {
    push_conflict(key, items, conflicts);
}

fn push_bool_conflict(
    key: &str,
    items: &[(Provenance, BoolAssertion)],
    conflicts: &mut Vec<ConflictEntry>,
) {
    push_conflict(key, items, conflicts);
}

fn push_string_conflict(
    key: &str,
    items: &[(Provenance, StringAssertion)],
    conflicts: &mut Vec<ConflictEntry>,
) {
    push_conflict(key, items, conflicts);
}

fn push_conflict<T: core::fmt::Debug>(
    key: &str,
    items: &[(Provenance, T)],
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

impl EngineRequirement for ClippyTomlRequirements {
    fn engine_id(&self) -> &'static str {
        crate::ENGINE_ID
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
