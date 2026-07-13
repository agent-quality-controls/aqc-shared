//! Dependency conflict checks used during requirement merging.

#![cfg_attr(
    not(test),
    expect(
        clippy::missing_docs_in_private_items,
        reason = "Private aggregate conflict helpers are internal requirement steps."
    )
)]
#![allow(
    clippy::type_complexity,
    reason = "Conflict helpers consume resolved dependency and forbidden-glob requirement shapes."
)]

use std::collections::{BTreeMap, BTreeSet};

use aqc_file_engine_core::{
    ConflictEntry, Provenance, ResolvedForbiddenGlobRequirements, ResolvedItemRequirements,
};
use globset::GlobBuilder;

use super::super::dependencies::{
    DependencyIdentity, DependencyPackageGlob, DependencyRequirement,
};
pub(super) fn push_dependency_identity_overlaps(
    key: &str,
    merged: &ResolvedItemRequirements<DependencyRequirement>,
    conflicts: &mut Vec<ConflictEntry>,
) {
    push_dependency_file_key_package_conflicts(key, merged, conflicts);

    for (identity, requirement) in &merged.required {
        if matches!(identity, DependencyIdentity::Invalid) {
            conflicts.push(ConflictEntry {
                key: format!("{key}.<invalid>"),
                reason: "invalid-dependency-requirement".to_owned(),
                contributors: requirement
                    .collected
                    .iter()
                    .map(|(prov, _)| (prov.clone(), "missing file_key or package".to_owned()))
                    .collect(),
            });
        }
    }
    for (identity, requirement) in &merged.forbidden {
        if matches!(identity, DependencyIdentity::Invalid) {
            conflicts.push(ConflictEntry {
                key: format!("{key}.<invalid>"),
                reason: "invalid-dependency-requirement".to_owned(),
                contributors: requirement
                    .collected
                    .iter()
                    .map(|(prov, _)| (prov.clone(), "missing file_key or package".to_owned()))
                    .collect(),
            });
        }
    }
}

fn push_dependency_file_key_package_conflicts(
    key: &str,
    merged: &ResolvedItemRequirements<DependencyRequirement>,
    conflicts: &mut Vec<ConflictEntry>,
) {
    let mut packages_by_key: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let mut contributors_by_key: BTreeMap<String, Vec<(Provenance, String)>> = BTreeMap::new();
    for requirement in merged.required.values() {
        let Some(file_key) = requirement.merged.file_key.as_ref() else {
            continue;
        };
        let effective_package = requirement
            .merged
            .value
            .package
            .as_deref()
            .unwrap_or(file_key)
            .to_owned();
        let _ = packages_by_key
            .entry(file_key.clone())
            .or_default()
            .insert(effective_package.clone());
        contributors_by_key
            .entry(file_key.clone())
            .or_default()
            .extend(requirement.collected.iter().map(|(prov, _)| {
                (
                    prov.clone(),
                    format!("file_key {file_key} package {effective_package}"),
                )
            }));
    }
    for (file_key, packages) in packages_by_key {
        if packages.len() <= 1 {
            continue;
        }
        conflicts.push(ConflictEntry {
            key: format!("{key}.{file_key}"),
            reason: "dependency-file-key-package-conflict".to_owned(),
            contributors: contributors_by_key.remove(&file_key).unwrap_or_default(),
        });
    }
}

pub(super) fn push_dependency_package_glob_conflicts(
    key: &str,
    merged: &ResolvedItemRequirements<DependencyRequirement>,
    globs: &ResolvedForbiddenGlobRequirements<DependencyPackageGlob>,
    conflicts: &mut Vec<ConflictEntry>,
) {
    for glob in globs.globs.values() {
        let Ok(compiled_glob) = GlobBuilder::new(&glob.merged.glob)
            .literal_separator(true)
            .build()
        else {
            continue;
        };
        let matcher = compiled_glob.compile_matcher();
        for (_, requirement) in merged.asserted_items() {
            let Some(package) = required_dependency_package(&requirement.merged) else {
                continue;
            };
            if !matcher.is_match(&package) {
                continue;
            }
            let mut contributors = requirement
                .collected
                .iter()
                .map(|(prov, _)| (prov.clone(), format!("required package {package}")))
                .collect::<Vec<_>>();
            contributors.extend(glob.collected.iter().map(|(prov, _)| {
                (
                    prov.clone(),
                    format!("forbidden package glob {}", glob.merged.glob),
                )
            }));
            conflicts.push(ConflictEntry {
                key: format!("{key}.{package}"),
                reason: "dependency-package-glob-forbids-required-package".to_owned(),
                contributors,
            });
        }
    }
}

fn required_dependency_package(requirement: &DependencyRequirement) -> Option<String> {
    requirement
        .value
        .package
        .clone()
        .or_else(|| requirement.file_key.clone())
}
