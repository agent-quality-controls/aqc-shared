//! `[profile.<name>]` requirements.

#![cfg_attr(
    not(test),
    expect(
        clippy::missing_docs_in_private_items,
        reason = "Private profile composition helpers are internal requirement steps."
    )
)]
#![expect(
    clippy::indexing_slicing,
    clippy::manual_retain,
    clippy::type_complexity,
    clippy::unnecessary_find_map,
    clippy::use_self,
    clippy::wildcard_enum_match_arm,
    reason = "Profile requirement composition uses closed local assertion enums and collected provenance tuples."
)]

use std::collections::BTreeMap;

use aqc_file_engine_core::{
    ConfigScalar, ConflictEntry, OnEmpty, OnEmptyClass, Provenance, Resolve, ResolvedRequirement,
    resolve_map,
};

use super::helpers::push_debug_conflict;

/// What must hold about a single profile field.
#[derive(Debug, Clone)]
pub enum ProfileFieldAssertion {
    Equals(ConfigScalar, String),
    OneOf(Vec<ConfigScalar>, String),
    Present(String),
    Absent(String),
}

impl PartialEq for ProfileFieldAssertion {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Equals(a, _), Self::Equals(b, _)) => a == b,
            (Self::OneOf(a, _), Self::OneOf(b, _)) => a == b,
            (Self::Present(_), Self::Present(_)) | (Self::Absent(_), Self::Absent(_)) => true,
            _ => false,
        }
    }
}

impl Resolve for ProfileFieldAssertion {
    type Merged = Self;

    fn resolve(
        key: &str,
        items: Vec<(Provenance, Self)>,
        conflicts: &mut Vec<ConflictEntry>,
    ) -> Option<ResolvedRequirement<Self::Merged, Self>> {
        if items.iter().any(|(_, a)| matches!(a, Self::Absent(_))) {
            if items.iter().all(|(_, a)| matches!(a, Self::Absent(_))) {
                return Some(ResolvedRequirement {
                    merged: Self::Absent(first_msg(&items)),
                    collected: items,
                });
            }
            push_conflict(key, &items, conflicts);
            return None;
        }
        resolve_scalar_assertions(key, items, conflicts)
    }
}

impl OnEmptyClass for ProfileFieldAssertion {
    fn on_empty(&self) -> OnEmpty {
        match self {
            Self::Equals(..) | Self::Absent(..) => OnEmpty::Writes,
            Self::OneOf(..) | Self::Present(..) => OnEmpty::ChecksOnly,
        }
    }
}

fn resolve_scalar_assertions(
    key: &str,
    items: Vec<(Provenance, ProfileFieldAssertion)>,
    conflicts: &mut Vec<ConflictEntry>,
) -> Option<ResolvedRequirement<ProfileFieldAssertion, ProfileFieldAssertion>> {
    let equals = items
        .iter()
        .filter_map(|(_, assertion)| match assertion {
            ProfileFieldAssertion::Equals(value, msg) => Some((value.clone(), msg.clone())),
            _ => None,
        })
        .collect::<Vec<_>>();
    let oneof = intersect_oneofs(
        items
            .iter()
            .filter_map(|(_, assertion)| match assertion {
                ProfileFieldAssertion::OneOf(values, msg) => Some((values.clone(), msg.clone())),
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
            .is_some_and(|(allowed, _)| !allowed.contains(value))
        {
            push_conflict(key, &items, conflicts);
            return None;
        }
        ProfileFieldAssertion::Equals(value.clone(), msg.clone())
    } else if let Some((allowed, msg)) = oneof {
        if allowed.is_empty() {
            push_conflict(key, &items, conflicts);
            return None;
        }
        ProfileFieldAssertion::OneOf(allowed, msg)
    } else {
        ProfileFieldAssertion::Present(first_msg(&items))
    };

    Some(ResolvedRequirement {
        merged,
        collected: items,
    })
}

fn intersect_oneofs(
    oneofs: Vec<(Vec<ConfigScalar>, String)>,
) -> Option<(Vec<ConfigScalar>, String)> {
    let mut iter = oneofs.into_iter();
    let (mut out, msg) = iter.next()?;
    for (next, _) in iter {
        out = out.into_iter().filter(|item| next.contains(item)).collect();
    }
    Some((out, msg))
}

fn first_msg(items: &[(Provenance, ProfileFieldAssertion)]) -> String {
    items
        .iter()
        .find_map(|(_, assertion)| match assertion {
            ProfileFieldAssertion::Equals(_, msg)
            | ProfileFieldAssertion::OneOf(_, msg)
            | ProfileFieldAssertion::Present(msg)
            | ProfileFieldAssertion::Absent(msg) => Some(msg.clone()),
        })
        .unwrap_or_default()
}

fn push_conflict(
    key: &str,
    items: &[(Provenance, ProfileFieldAssertion)],
    conflicts: &mut Vec<ConflictEntry>,
) {
    push_debug_conflict(key, "scalar-disagree", items, conflicts);
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct ProfileRequirements {
    pub fields: BTreeMap<String, ProfileFieldAssertion>,
    pub package_overrides: BTreeMap<String, ProfileRequirements>,
    pub build_override: Option<Box<ProfileRequirements>>,
}

#[derive(Debug, Clone, Default)]
pub struct ResolvedProfileRequirements {
    pub fields: BTreeMap<String, ResolvedRequirement<ProfileFieldAssertion, ProfileFieldAssertion>>,
    pub package_overrides: BTreeMap<String, ResolvedProfileRequirements>,
    pub build_override: Option<Box<ResolvedProfileRequirements>>,
}

impl ProfileRequirements {
    pub fn resolve(
        key: &str,
        items: Vec<(Provenance, Self)>,
        conflicts: &mut Vec<ConflictEntry>,
    ) -> ResolvedProfileRequirements {
        let mut fields = Vec::new();
        let mut overrides: BTreeMap<String, Vec<(Provenance, ProfileRequirements)>> =
            BTreeMap::new();
        let mut build = Vec::new();

        for (prov, profile) in items {
            fields.push((prov.clone(), profile.fields));
            for (name, nested) in profile.package_overrides {
                overrides
                    .entry(name)
                    .or_default()
                    .push((prov.clone(), nested));
            }
            if let Some(nested) = profile.build_override {
                build.push((prov, *nested));
            }
        }

        let mut package_overrides = BTreeMap::new();
        for (name, nested) in overrides {
            let nested_key = format!("{key}.package.{name}");
            let _ = package_overrides.insert(name, Self::resolve(&nested_key, nested, conflicts));
        }

        let build_override = if build.is_empty() {
            None
        } else {
            Some(Box::new(Self::resolve(
                &format!("{key}.build-override"),
                build,
                conflicts,
            )))
        };

        ResolvedProfileRequirements {
            fields: resolve_map(fields, |field| format!("{key}.{field}"), conflicts),
            package_overrides,
            build_override,
        }
    }
}
