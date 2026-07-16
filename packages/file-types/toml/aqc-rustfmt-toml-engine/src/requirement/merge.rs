//! Rustfmt requirement merge logic.

use std::collections::BTreeMap;

use aqc_file_engine_core::{
    ConfigScalar, ConflictEntry, FileKeyRequirement, ListRequirements, Provenance, Resolve,
    ResolvedForbiddenGlobRequirements, ResolvedListRequirements, ScalarAssertion,
    resolve_key_membership,
};
use globset::GlobBuilder;

use super::{
    ResolvedRustfmtScalarSettings, ResolvedRustfmtTomlRequirements, RustfmtIgnorePathGlob,
    RustfmtListSetting, RustfmtScalarRequirements, RustfmtScalarSetting, RustfmtTomlRequirements,
};

/// Raw rustfmt requirement inputs grouped with provenance.
type RustfmtRequirementInput = Vec<(Provenance, RustfmtTomlRequirements)>;

/// List setting requirements grouped by rustfmt list setting.
type RustfmtListRequirementsByKey =
    BTreeMap<RustfmtListSetting, Vec<(Provenance, ListRequirements)>>;
/// Scalar setting requirements grouped by provenance.
type RustfmtScalarRequirementInput = Vec<(Provenance, RustfmtScalarRequirements)>;
/// Scalar assertions grouped by rustfmt setting.
type RustfmtScalarAssertionsByKey =
    BTreeMap<RustfmtScalarSetting, Vec<(Provenance, ScalarAssertion<ConfigScalar>)>>;
impl RustfmtTomlRequirements {
    /// Merges all rustfmt TOML requirements into one resolved requirement set.
    ///
    /// # Errors
    ///
    /// Returns every conflict when the input requirements cannot be composed.
    #[allow(
        clippy::type_complexity,
        reason = "The explicit result names the resolved root and complete conflict collection."
    )]
    pub fn merge(
        reqs: RustfmtRequirementInput,
    ) -> Result<ResolvedRustfmtTomlRequirements, Vec<ConflictEntry>> {
        let mut conflicts = Vec::new();
        let scalar_settings = resolve_scalar_settings(
            reqs.iter()
                .map(|(prov, req)| (prov.clone(), req.scalar_settings.clone()))
                .collect(),
            &mut conflicts,
        );
        let forbidden_ignore_path_globs = aqc_file_engine_core::resolve_forbidden_globs(
            RustfmtListSetting::Ignore.file_key(),
            reqs.iter()
                .map(|(prov, req)| (prov.clone(), req.forbidden_ignore_path_globs.clone()))
                .collect(),
            &mut conflicts,
        );
        let setting_keys = resolve_key_membership(
            "rustfmt.toml",
            reqs.iter()
                .map(|(provenance, requirement)| {
                    (provenance.clone(), requirement.setting_keys.clone())
                })
                .collect(),
            reqs.iter()
                .map(|(provenance, requirement)| {
                    (
                        provenance.clone(),
                        setting_key_constraints(requirement),
                    )
                })
                .collect(),
            &mut conflicts,
        );

        let mut lists_by_key: RustfmtListRequirementsByKey = BTreeMap::new();
        for (prov, req) in reqs {
            for (key, list) in req.list_settings {
                lists_by_key
                    .entry(key)
                    .or_default()
                    .push((prov.clone(), list));
            }
        }

        let mut list_settings = BTreeMap::new();
        for (key, lists) in lists_by_key {
            let _ = list_settings.insert(
                key,
                aqc_file_engine_core::resolve_list(key.file_key(), lists, &mut conflicts),
            );
        }
        if let Some(ignore) = list_settings.get(&RustfmtListSetting::Ignore) {
            push_ignore_glob_conflicts(ignore, &forbidden_ignore_path_globs, &mut conflicts);
        }

        let resolved = ResolvedRustfmtTomlRequirements {
            scalar_settings,
            list_settings,
            forbidden_ignore_path_globs,
            setting_keys,
        };

        if conflicts.is_empty() {
            Ok(resolved)
        } else {
            Err(conflicts)
        }
    }
}

fn setting_key_constraints(
    requirement: &RustfmtTomlRequirements,
) -> aqc_file_engine_core::ItemRequirements<aqc_file_engine_core::KeyedItem<()>> {
    let mut constraints = aqc_file_engine_core::ItemRequirements::default();
    for (setting, assertion) in &requirement.scalar_settings {
        assertion.constrain_file_key(setting.file_key(), &mut constraints);
    }
    for (setting, list) in &requirement.list_settings {
        list.constrain_file_key(setting.file_key(), &mut constraints);
    }
    constraints
}

/// Resolves rustfmt scalar settings after checking setting-specific legality.
fn resolve_scalar_settings(
    input: RustfmtScalarRequirementInput,
    conflicts: &mut Vec<ConflictEntry>,
) -> ResolvedRustfmtScalarSettings {
    let mut by_key: RustfmtScalarAssertionsByKey = BTreeMap::new();
    for (prov, map) in input {
        for (key, assertion) in map {
            by_key
                .entry(key)
                .or_default()
                .push((prov.clone(), assertion));
        }
    }

    let mut out = BTreeMap::new();
    for (key, items) in by_key {
        let key_text = key.file_key();
        if !items
            .iter()
            .all(|(_, assertion)| key.scalar_assertion_is_legal(assertion))
        {
            aqc_file_engine_core::push_conflict(
                key_text,
                "scalar-operation-unsupported",
                &items,
                aqc_file_engine_core::render_scalar_assertion,
                conflicts,
            );
            continue;
        }
        if let Some(resolved) = ScalarAssertion::<ConfigScalar>::resolve(key_text, items, conflicts)
        {
            let _ = out.insert(key, resolved);
        }
    }
    out
}

/// Records conflicts between required ignore entries and forbidden path globs.
fn push_ignore_glob_conflicts(
    ignore: &ResolvedListRequirements,
    globs: &ResolvedForbiddenGlobRequirements<RustfmtIgnorePathGlob>,
    conflicts: &mut Vec<ConflictEntry>,
) {
    for glob in globs.globs.values() {
        let Ok(compiled) = GlobBuilder::new(&glob.merged.glob).build() else {
            continue;
        };
        let matcher = compiled.compile_matcher();
        let required = ignore
            .contains
            .keys()
            .chain(ignore.exact.iter().flat_map(|exact| exact.merged.iter()))
            .filter(|path| matcher.is_match(path.as_str()))
            .cloned()
            .collect::<std::collections::BTreeSet<_>>();
        if required.is_empty() {
            continue;
        }
        for path in required {
            let mut contributors = ignore
                .contains
                .get(&path)
                .into_iter()
                .flat_map(|req| req.collected.iter())
                .map(|(prov, _)| (prov.clone(), "required".to_owned()))
                .collect::<Vec<_>>();
            contributors.extend(
                ignore
                    .exact
                    .iter()
                    .flat_map(|req| req.collected.iter())
                    .filter(|(_, (values, _))| values.iter().any(|value| value == &path))
                    .map(|(prov, _)| (prov.clone(), "required".to_owned())),
            );
            contributors.extend(
                glob.collected
                    .iter()
                    .map(|(prov, _)| (prov.clone(), "forbidden".to_owned())),
            );
            conflicts.push(ConflictEntry {
                key: format!("{}.{}", RustfmtListSetting::Ignore.file_key(), path),
                reason: "ignore-path-glob-forbids-required-path".to_owned(),
                contributors,
            });
        }
    }
}
