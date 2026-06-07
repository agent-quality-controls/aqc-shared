//! `[profile.<name>]` assertions, including `[profile.<n>.package.<spec>]`
//! overrides and `build-override`.

use std::collections::BTreeMap;

use aqc_file_engine_core::{
    ConfigScalar, ConflictEntry, Msg, OnEmpty, OnEmptyClass, Provenance, Resolve, merge_map,
};

/// What must hold about a single profile field (`opt-level`, `lto`, ...).
///
/// Equality (and therefore merge agreement) ignores the policy message.
#[derive(Debug, Clone)]
pub enum ProfileFieldAssertion {
    /// The field equals this value (`opt-level = 3`, `lto = "thin"`).
    Equals(ConfigScalar, Msg),
    /// The field's value is one of these (check-only).
    OneOf(Vec<ConfigScalar>, Msg),
    /// The field is set, to anything (check-only).
    Present(Msg),
    /// The field is not set.
    Absent(Msg),
}

/// Semantic equality: messages excluded.
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

impl OnEmptyClass for ProfileFieldAssertion {
    fn on_empty(&self) -> OnEmpty {
        match self {
            Self::Equals(..) | Self::Absent(..) => OnEmpty::Writes,
            Self::OneOf(..) | Self::Present(..) => OnEmpty::ChecksOnly,
        }
    }
}

/// One profile table's field assertions, keyed by field name.
pub type ProfileFields = BTreeMap<String, ProfileFieldAssertion>;

/// Per-package-spec override contributions gathered during resolve.
type OverrideContributions = BTreeMap<String, Vec<(Provenance, ProfileFields)>>;

/// What must hold about a `[profile.<name>]` table: its direct fields, its
/// per-package overrides, and its `build-override` table.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ProfileAssertion {
    /// Direct `[profile.<name>].<field>` assertions, keyed by field.
    pub fields: ProfileFields,
    /// `[profile.<name>.package."<spec>"].<field>`, keyed by package spec
    /// then field.
    pub package_overrides: BTreeMap<String, ProfileFields>,
    /// `[profile.<name>.build-override].<field>`, keyed by field.
    pub build_override: ProfileFields,
}

impl Resolve for ProfileAssertion {
    fn resolve(
        key: &str,
        contributions: Vec<(Provenance, Self)>,
        conflicts: &mut Vec<ConflictEntry>,
    ) -> Option<Self> {
        // Union the three maps per key; per-key disagreement conflicts.
        let mut fields_contribs = Vec::new();
        let mut build_contribs = Vec::new();
        let mut override_contribs: OverrideContributions = BTreeMap::new();
        for (prov, a) in contributions {
            fields_contribs.push((prov.clone(), a.fields));
            build_contribs.push((prov.clone(), a.build_override));
            for (spec, map) in a.package_overrides {
                override_contribs
                    .entry(spec)
                    .or_default()
                    .push((prov.clone(), map));
            }
        }
        let render = |a: &ProfileFieldAssertion| format!("{a:?}");
        let fields = merge_map(key, fields_contribs, render, conflicts);
        let build_override = merge_map(
            &format!("{key}.build-override"),
            build_contribs,
            render,
            conflicts,
        );
        let mut package_overrides = BTreeMap::new();
        for (spec, contribs) in override_contribs {
            let merged = merge_map(
                &format!("{key}.package.{spec}"),
                contribs,
                render,
                conflicts,
            );
            let _ = package_overrides.insert(spec, merged);
        }
        Some(Self {
            fields,
            package_overrides,
            build_override,
        })
    }
}

impl OnEmptyClass for ProfileAssertion {
    fn on_empty(&self) -> OnEmpty {
        // Writable when every contained field assertion is writable.
        let all_fields = self
            .fields
            .values()
            .chain(self.build_override.values())
            .chain(self.package_overrides.values().flat_map(BTreeMap::values));
        for a in all_fields {
            if a.on_empty() == OnEmpty::ChecksOnly {
                return OnEmpty::ChecksOnly;
            }
        }
        OnEmpty::Writes
    }
}
