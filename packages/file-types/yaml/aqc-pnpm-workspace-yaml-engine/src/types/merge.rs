//! Requirement composition and cross-collection conflict checks.

use std::collections::BTreeMap;

use aqc_file_engine_core::{
    ConflictEntry, Provenance, ResolvedForbiddenGlobRequirements, ResolvedItemRequirements,
    ResolvedListRequirements, ScalarAssertion, asserted_items, resolve_forbidden_globs,
    resolve_items, resolve_list, resolve_maybe,
};

use crate::runtime::selector_matches;
use crate::types::{
    PnpmPackageSelectorGlob, PnpmWorkspaceYamlRequirements, ResolvedPnpmWorkspaceYamlRequirements,
};

impl PnpmWorkspaceYamlRequirements {
    /// Composes provenance-tagged pnpm requirements.
    ///
    /// # Errors
    ///
    /// Returns every scalar, collection, and forbidden-selector conflict.
    #[expect(
        clippy::needless_pass_by_value,
        reason = "The shared merged_reconcile contract supplies an owned requirement vector to every engine merge function."
    )]
    pub fn merge(
        requirements: Vec<(Provenance, Self)>,
    ) -> Result<ResolvedPnpmWorkspaceYamlRequirements, Vec<ConflictEntry>> {
        let mut conflicts = Vec::new();
        let mut exact_settings = requirements
            .iter()
            .filter_map(|(provenance, requirement)| {
                requirement
                    .exact_settings
                    .clone()
                    .map(|message| (provenance.clone(), message))
            })
            .collect::<Vec<_>>();
        exact_settings.sort_by(|left, right| left.0.cmp(&right.0));
        let resolved = ResolvedPnpmWorkspaceYamlRequirements {
            strict_peer_dependencies: scalar(
                "strictPeerDependencies",
                &requirements,
                |r| r.strict_peer_dependencies.clone(),
                &mut conflicts,
            ),
            engine_strict: scalar(
                "engineStrict",
                &requirements,
                |r| r.engine_strict.clone(),
                &mut conflicts,
            ),
            minimum_release_age: scalar(
                "minimumReleaseAge",
                &requirements,
                |r| r.minimum_release_age.clone(),
                &mut conflicts,
            ),
            minimum_release_age_strict: scalar(
                "minimumReleaseAgeStrict",
                &requirements,
                |r| r.minimum_release_age_strict.clone(),
                &mut conflicts,
            ),
            minimum_release_age_ignore_missing_time: scalar(
                "minimumReleaseAgeIgnoreMissingTime",
                &requirements,
                |r| r.minimum_release_age_ignore_missing_time.clone(),
                &mut conflicts,
            ),
            minimum_release_age_exclude: list(
                "minimumReleaseAgeExclude",
                &requirements,
                |r| r.minimum_release_age_exclude.clone(),
                &mut conflicts,
            ),
            forbidden_minimum_release_age_exclude_globs: globs(
                "minimumReleaseAgeExclude",
                &requirements,
                |r| r.forbidden_minimum_release_age_exclude_globs.clone(),
                &mut conflicts,
            ),
            trust_policy: scalar(
                "trustPolicy",
                &requirements,
                |r| r.trust_policy.clone(),
                &mut conflicts,
            ),
            trust_lockfile: scalar(
                "trustLockfile",
                &requirements,
                |r| r.trust_lockfile.clone(),
                &mut conflicts,
            ),
            trust_policy_ignore_after: scalar(
                "trustPolicyIgnoreAfter",
                &requirements,
                |r| r.trust_policy_ignore_after.clone(),
                &mut conflicts,
            ),
            trust_policy_exclude: list(
                "trustPolicyExclude",
                &requirements,
                |r| r.trust_policy_exclude.clone(),
                &mut conflicts,
            ),
            forbidden_trust_policy_exclude_globs: globs(
                "trustPolicyExclude",
                &requirements,
                |r| r.forbidden_trust_policy_exclude_globs.clone(),
                &mut conflicts,
            ),
            block_exotic_subdeps: scalar(
                "blockExoticSubdeps",
                &requirements,
                |r| r.block_exotic_subdeps.clone(),
                &mut conflicts,
            ),
            pm_on_fail: scalar(
                "pmOnFail",
                &requirements,
                |r| r.pm_on_fail.clone(),
                &mut conflicts,
            ),
            strict_dep_builds: scalar(
                "strictDepBuilds",
                &requirements,
                |r| r.strict_dep_builds.clone(),
                &mut conflicts,
            ),
            dangerously_allow_all_builds: scalar(
                "dangerouslyAllowAllBuilds",
                &requirements,
                |r| r.dangerously_allow_all_builds.clone(),
                &mut conflicts,
            ),
            allow_builds: items("allowBuilds", &requirements, &mut conflicts),
            forbidden_allowed_build_package_globs: globs(
                "allowBuilds",
                &requirements,
                |r| r.forbidden_allowed_build_package_globs.clone(),
                &mut conflicts,
            ),
            exact_settings,
        };
        list_glob_conflicts(
            "minimumReleaseAgeExclude",
            &resolved.minimum_release_age_exclude,
            &resolved.forbidden_minimum_release_age_exclude_globs,
            &mut conflicts,
        );
        list_glob_conflicts(
            "trustPolicyExclude",
            &resolved.trust_policy_exclude,
            &resolved.forbidden_trust_policy_exclude_globs,
            &mut conflicts,
        );
        allow_build_glob_conflicts(
            &resolved.allow_builds,
            &resolved.forbidden_allowed_build_package_globs,
            &mut conflicts,
        );
        if conflicts.is_empty() {
            Ok(resolved)
        } else {
            Err(conflicts)
        }
    }
}

fn scalar<T: aqc_file_engine_core::ScalarValue>(
    key: &str,
    requirements: &[(Provenance, PnpmWorkspaceYamlRequirements)],
    field: impl Fn(&PnpmWorkspaceYamlRequirements) -> Option<ScalarAssertion<T>>,
    conflicts: &mut Vec<ConflictEntry>,
) -> Option<aqc_file_engine_core::ResolvedRequirement<ScalarAssertion<T>, ScalarAssertion<T>>> {
    resolve_maybe(
        key,
        requirements
            .iter()
            .map(|(p, r)| (p.clone(), field(r)))
            .collect(),
        conflicts,
    )
}

fn list(
    key: &str,
    requirements: &[(Provenance, PnpmWorkspaceYamlRequirements)],
    field: impl Fn(&PnpmWorkspaceYamlRequirements) -> aqc_file_engine_core::ListRequirements,
    conflicts: &mut Vec<ConflictEntry>,
) -> ResolvedListRequirements {
    resolve_list(
        key,
        requirements
            .iter()
            .map(|(p, r)| (p.clone(), field(r)))
            .collect(),
        conflicts,
    )
}

fn globs(
    key: &str,
    requirements: &[(Provenance, PnpmWorkspaceYamlRequirements)],
    field: impl Fn(
        &PnpmWorkspaceYamlRequirements,
    ) -> aqc_file_engine_core::ForbiddenGlobRequirements<PnpmPackageSelectorGlob>,
    conflicts: &mut Vec<ConflictEntry>,
) -> ResolvedForbiddenGlobRequirements<PnpmPackageSelectorGlob> {
    resolve_forbidden_globs(
        key,
        requirements
            .iter()
            .map(|(p, r)| (p.clone(), field(r)))
            .collect(),
        conflicts,
    )
}

fn items(
    key: &str,
    requirements: &[(Provenance, PnpmWorkspaceYamlRequirements)],
    conflicts: &mut Vec<ConflictEntry>,
) -> ResolvedItemRequirements<aqc_file_engine_core::KeyedItem<bool>> {
    resolve_items(
        key,
        requirements
            .iter()
            .map(|(p, r)| (p.clone(), r.allow_builds.clone()))
            .collect(),
        conflicts,
    )
}

fn list_glob_conflicts(
    key: &str,
    list: &ResolvedListRequirements,
    globs: &ResolvedForbiddenGlobRequirements<PnpmPackageSelectorGlob>,
    conflicts: &mut Vec<ConflictEntry>,
) {
    let mut contributors_by_value: BTreeMap<String, Vec<(Provenance, String)>> = BTreeMap::new();
    for (value, required) in &list.contains {
        contributors_by_value
            .entry(value.clone())
            .or_default()
            .extend(required.collected.clone());
    }
    if let Some(exact) = &list.exact {
        for value in &exact.merged {
            contributors_by_value
                .entry(value.clone())
                .or_default()
                .extend(
                    exact
                        .collected
                        .iter()
                        .map(|(provenance, (_, message))| (provenance.clone(), message.clone())),
                );
        }
    }
    for (value, mut contributors) in contributors_by_value {
        contributors.sort();
        contributors.dedup();
        push_matching_conflict(key, &value, globs, contributors, conflicts);
    }
}

fn allow_build_glob_conflicts(
    items: &ResolvedItemRequirements<aqc_file_engine_core::KeyedItem<bool>>,
    globs: &ResolvedForbiddenGlobRequirements<PnpmPackageSelectorGlob>,
    conflicts: &mut Vec<ConflictEntry>,
) {
    for (_, item) in asserted_items(items).filter(|(_, item)| item.merged.value) {
        let contributors = item
            .collected
            .iter()
            .map(|(p, (_, message))| (p.clone(), message.clone()))
            .collect();
        push_matching_conflict(
            "allowBuilds",
            &item.merged.file_key,
            globs,
            contributors,
            conflicts,
        );
    }
}

fn push_matching_conflict(
    key: &str,
    value: &str,
    globs: &ResolvedForbiddenGlobRequirements<PnpmPackageSelectorGlob>,
    mut contributors: Vec<(Provenance, String)>,
    conflicts: &mut Vec<ConflictEntry>,
) {
    let required_count = contributors.len();
    for glob in globs.globs.values() {
        if selector_matches(&glob.merged.glob, value).unwrap_or(false) {
            contributors.extend(glob.collected.iter().cloned());
        }
    }
    if contributors.len() > required_count {
        conflicts.push(ConflictEntry {
            key: format!("{key}.{value}"),
            reason: "required-forbidden-glob".to_owned(),
            contributors,
        });
    }
}
