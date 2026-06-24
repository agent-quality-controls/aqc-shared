//! `[profile.<name>]` requirements.

#![expect(
    clippy::type_complexity,
    clippy::use_self,
    reason = "Profile requirement composition uses core scalar assertions and collected provenance tuples."
)]

use std::collections::BTreeMap;

use aqc_file_engine_core::{
    ConfigScalar, ConflictEntry, Provenance, ResolvedRequirement, ScalarAssertion, resolve_map,
};

#[derive(Debug, Clone, Default, PartialEq)]
pub struct ProfileRequirements {
    pub fields: BTreeMap<String, ScalarAssertion<ConfigScalar>>,
    pub package_overrides: BTreeMap<String, ProfileRequirements>,
    pub build_override: Option<Box<ProfileRequirements>>,
}

#[derive(Debug, Clone, Default)]
pub struct ResolvedProfileRequirements {
    pub fields: BTreeMap<
        String,
        ResolvedRequirement<ScalarAssertion<ConfigScalar>, ScalarAssertion<ConfigScalar>>,
    >,
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
