//! `Cargo.toml` requirement aggregate and merge phase.

#![expect(
    clippy::disallowed_types,
    reason = "`Any` is used only for EngineRequirement downcast dispatch."
)]

use core::any::Any;
use std::collections::{BTreeMap, BTreeSet};

use aqc_file_engine_core::{
    ConflictEntry, EngineRequirement, FileItemRequirement, ForbiddenGlobRequirement,
    ForbiddenGlobRequirements, ItemRequirements, KeyedItem, Provenance,
    ResolvedForbiddenGlobRequirements, ResolvedItemRequirements, ResolvedRequirement,
    resolve_forbidden_globs, resolve_items, resolve_map,
};
use globset::GlobBuilder;

use super::dependencies::{
    DependencyIdentity, DependencyPackageGlob, DependencyRequirement, DependencyScope,
};
use super::features::FeatureMembers;
use super::lints::{LintSetting, PackageLintsAssertion, ResolvedPackageLintsAssertion};
use super::package::{PackageFieldAssertion, ResolvedPackageFieldAssertion};
use super::profiles::{ProfileRequirements, ResolvedProfileRequirements};
use super::sections::{ManifestSection, SectionPresenceAssertion};
use super::targets::ResolvedTargetRequirements;
use super::targets::TargetRequirements;
use super::workspace::{ResolvedWorkspaceFieldAssertion, WorkspaceFieldAssertion};

#[derive(Debug, Clone, Default)]
pub struct CargoTomlRequirements {
    pub package_lints: Option<PackageLintsAssertion>,
    pub workspace_lints: BTreeMap<String, ItemRequirements<KeyedItem<LintSetting>>>,
    pub package_fields: BTreeMap<String, PackageFieldAssertion>,
    pub workspace_package_fields: BTreeMap<String, PackageFieldAssertion>,
    pub workspace_fields: BTreeMap<String, WorkspaceFieldAssertion>,
    pub section_presence: BTreeMap<ManifestSection, SectionPresenceAssertion>,
    pub dependencies: BTreeMap<DependencyScope, ItemRequirements<DependencyRequirement>>,
    pub forbidden_dependency_package_globs:
        BTreeMap<DependencyScope, ForbiddenGlobRequirements<DependencyPackageGlob>>,
    pub workspace_dependencies: Option<ItemRequirements<DependencyRequirement>>,
    pub forbidden_workspace_dependency_package_globs:
        Option<ForbiddenGlobRequirements<DependencyPackageGlob>>,
    pub features: Option<ItemRequirements<KeyedItem<FeatureMembers>>>,
    pub profiles: BTreeMap<String, ProfileRequirements>,
    pub targets: TargetRequirements,
    pub patch: BTreeMap<String, ItemRequirements<DependencyRequirement>>,
    pub forbidden_patch_dependency_package_globs:
        BTreeMap<String, ForbiddenGlobRequirements<DependencyPackageGlob>>,
}

#[rustfmt::skip]
#[derive(Debug, Clone, Default)]
pub struct ResolvedCargoTomlRequirements {
    pub package_lints: Option<ResolvedPackageLintsAssertion>,
    pub workspace_lints: BTreeMap<String, ResolvedItemRequirements<KeyedItem<LintSetting>>>,
    pub package_fields:
        BTreeMap<String, ResolvedRequirement<ResolvedPackageFieldAssertion, PackageFieldAssertion>>,
    pub workspace_package_fields:
        BTreeMap<String, ResolvedRequirement<ResolvedPackageFieldAssertion, PackageFieldAssertion>>,
    pub workspace_fields:
        BTreeMap<String, ResolvedRequirement<ResolvedWorkspaceFieldAssertion, WorkspaceFieldAssertion>>,
    pub section_presence: BTreeMap<ManifestSection, ResolvedRequirement<SectionPresenceAssertion, SectionPresenceAssertion>>,
    pub dependencies: BTreeMap<DependencyScope, ResolvedItemRequirements<DependencyRequirement>>,
    pub forbidden_dependency_package_globs:
        BTreeMap<DependencyScope, ResolvedForbiddenGlobRequirements<DependencyPackageGlob>>,
    pub dependency_glob_conflicts:
        BTreeMap<DependencyScope, DependencyForbiddenGlobConflictBlocks>,
    pub workspace_dependencies: Option<ResolvedItemRequirements<DependencyRequirement>>,
    pub forbidden_workspace_dependency_package_globs:
        Option<ResolvedForbiddenGlobRequirements<DependencyPackageGlob>>,
    pub workspace_dependency_glob_conflicts: DependencyForbiddenGlobConflictBlocks,
    pub features: Option<ResolvedItemRequirements<KeyedItem<FeatureMembers>>>,
    pub profiles: BTreeMap<String, ResolvedProfileRequirements>,
    pub targets: ResolvedTargetRequirements,
    pub patch: BTreeMap<String, ResolvedItemRequirements<DependencyRequirement>>,
    pub forbidden_patch_dependency_package_globs:
        BTreeMap<String, ResolvedForbiddenGlobRequirements<DependencyPackageGlob>>,
    pub patch_dependency_glob_conflicts:
        BTreeMap<String, DependencyForbiddenGlobConflictBlocks>,
}

#[derive(Debug, Clone, Default)]
pub struct DependencyForbiddenGlobConflictBlocks {
    pub required: BTreeSet<DependencyIdentity>,
    pub package_globs: BTreeSet<String>,
}

impl DependencyForbiddenGlobConflictBlocks {
    fn is_empty(&self) -> bool {
        self.required.is_empty() && self.package_globs.is_empty()
    }
}

impl CargoTomlRequirements {
    #[must_use]
    pub fn merge(
        reqs: Vec<(Provenance, CargoTomlRequirements)>,
    ) -> (ResolvedCargoTomlRequirements, Vec<ConflictEntry>) {
        let mut conflicts = Vec::new();

        let package_lints = PackageLintsAssertion::resolve(
            "[lints]",
            reqs.iter()
                .filter_map(|(prov, req)| {
                    req.package_lints
                        .clone()
                        .map(|assertion| (prov.clone(), assertion))
                })
                .collect(),
            &mut conflicts,
        );

        let workspace_lints = resolve_item_map(
            reqs.iter()
                .map(|(prov, req)| (prov.clone(), req.workspace_lints.clone()))
                .collect(),
            |tool| format!("[workspace.lints.{tool}]"),
            &mut conflicts,
        );

        let dependencies = resolve_item_map(
            reqs.iter()
                .map(|(prov, req)| (prov.clone(), req.dependencies.clone()))
                .collect(),
            DependencyScope::table_path,
            &mut conflicts,
        );
        for (scope, merged) in &dependencies {
            push_dependency_identity_overlaps(&scope.table_path(), merged, &mut conflicts);
        }

        let forbidden_dependency_package_globs = resolve_forbidden_glob_map(
            reqs.iter()
                .map(|(prov, req)| (prov.clone(), req.forbidden_dependency_package_globs.clone()))
                .collect(),
            DependencyScope::table_path,
            &mut conflicts,
        );
        let mut dependency_glob_conflicts = BTreeMap::new();
        for (scope, globs) in &forbidden_dependency_package_globs {
            let Some(merged) = dependencies.get(scope) else {
                continue;
            };
            let blocks = push_dependency_package_glob_conflicts(
                &scope.table_path(),
                merged,
                globs,
                &mut conflicts,
            );
            if !blocks.is_empty() {
                let _ = dependency_glob_conflicts.insert(scope.clone(), blocks);
            }
        }

        let patch = resolve_item_map(
            reqs.iter()
                .map(|(prov, req)| (prov.clone(), req.patch.clone()))
                .collect(),
            |registry| format!("[patch.{registry}]"),
            &mut conflicts,
        );
        for (registry, merged) in &patch {
            push_dependency_identity_overlaps(
                &format!("[patch.{registry}]"),
                merged,
                &mut conflicts,
            );
        }

        let forbidden_patch_dependency_package_globs = resolve_forbidden_glob_map(
            reqs.iter()
                .map(|(prov, req)| {
                    (
                        prov.clone(),
                        req.forbidden_patch_dependency_package_globs.clone(),
                    )
                })
                .collect(),
            |registry| format!("[patch.{registry}]"),
            &mut conflicts,
        );
        let mut patch_dependency_glob_conflicts = BTreeMap::new();
        for (registry, globs) in &forbidden_patch_dependency_package_globs {
            let Some(merged) = patch.get(registry) else {
                continue;
            };
            let blocks = push_dependency_package_glob_conflicts(
                &format!("[patch.{registry}]"),
                merged,
                globs,
                &mut conflicts,
            );
            if !blocks.is_empty() {
                let _ = patch_dependency_glob_conflicts.insert(registry.clone(), blocks);
            }
        }

        let profiles = resolve_profile_map(
            reqs.iter()
                .map(|(prov, req)| (prov.clone(), req.profiles.clone()))
                .collect(),
            &mut conflicts,
        );

        let targets = TargetRequirements::resolve(
            reqs.iter()
                .map(|(prov, req)| (prov.clone(), req.targets.clone()))
                .collect(),
            &mut conflicts,
        );

        let workspace_dependencies = resolve_maybe_items(
            "[workspace.dependencies]",
            reqs.iter()
                .map(|(prov, req)| (prov.clone(), req.workspace_dependencies.clone()))
                .collect(),
            &mut conflicts,
        );
        if let Some(merged) = &workspace_dependencies {
            push_dependency_identity_overlaps("[workspace.dependencies]", merged, &mut conflicts);
        }

        let forbidden_workspace_dependency_package_globs = resolve_maybe_forbidden_globs(
            "[workspace.dependencies]",
            reqs.iter()
                .map(|(prov, req)| {
                    (
                        prov.clone(),
                        req.forbidden_workspace_dependency_package_globs.clone(),
                    )
                })
                .collect(),
            &mut conflicts,
        );
        let mut workspace_dependency_glob_conflicts =
            DependencyForbiddenGlobConflictBlocks::default();
        if let (Some(merged), Some(globs)) = (
            &workspace_dependencies,
            &forbidden_workspace_dependency_package_globs,
        ) {
            workspace_dependency_glob_conflicts = push_dependency_package_glob_conflicts(
                "[workspace.dependencies]",
                merged,
                globs,
                &mut conflicts,
            );
        }

        let features = resolve_maybe_items(
            "[features]",
            reqs.iter()
                .map(|(prov, req)| (prov.clone(), req.features.clone()))
                .collect(),
            &mut conflicts,
        );

        let out = ResolvedCargoTomlRequirements {
            package_lints,
            workspace_lints,
            package_fields: resolve_map(
                reqs.iter()
                    .map(|(prov, req)| (prov.clone(), req.package_fields.clone()))
                    .collect(),
                |field| format!("[package].{field}"),
                &mut conflicts,
            ),
            workspace_package_fields: resolve_map(
                reqs.iter()
                    .map(|(prov, req)| (prov.clone(), req.workspace_package_fields.clone()))
                    .collect(),
                |field| format!("[workspace.package].{field}"),
                &mut conflicts,
            ),
            workspace_fields: resolve_map(
                reqs.iter()
                    .map(|(prov, req)| (prov.clone(), req.workspace_fields.clone()))
                    .collect(),
                |field| format!("[workspace].{field}"),
                &mut conflicts,
            ),
            section_presence: resolve_map(
                reqs.iter()
                    .map(|(prov, req)| (prov.clone(), req.section_presence.clone()))
                    .collect(),
                |section: &ManifestSection| section.table_path().to_owned(),
                &mut conflicts,
            ),
            dependencies,
            forbidden_dependency_package_globs,
            dependency_glob_conflicts,
            workspace_dependencies,
            forbidden_workspace_dependency_package_globs,
            workspace_dependency_glob_conflicts,
            features,
            profiles,
            targets,
            patch,
            forbidden_patch_dependency_package_globs,
            patch_dependency_glob_conflicts,
        };

        (out, conflicts)
    }
}

fn resolve_forbidden_glob_map<K, Glob>(
    input: Vec<(Provenance, BTreeMap<K, ForbiddenGlobRequirements<Glob>>)>,
    key_path: impl Fn(&K) -> String,
    conflicts: &mut Vec<ConflictEntry>,
) -> BTreeMap<K, ResolvedForbiddenGlobRequirements<Glob>>
where
    K: Ord + Clone,
    Glob: ForbiddenGlobRequirement,
    Glob::Identity: ToString,
{
    let mut by_key: BTreeMap<K, Vec<(Provenance, ForbiddenGlobRequirements<Glob>)>> =
        BTreeMap::new();
    for (prov, map) in input {
        for (key, globs) in map {
            by_key.entry(key).or_default().push((prov.clone(), globs));
        }
    }
    by_key
        .into_iter()
        .map(|(key, globs)| {
            let path = key_path(&key);
            (key, resolve_forbidden_globs(&path, globs, conflicts))
        })
        .collect()
}

fn resolve_maybe_forbidden_globs<Glob>(
    key: &str,
    input: Vec<(Provenance, Option<ForbiddenGlobRequirements<Glob>>)>,
    conflicts: &mut Vec<ConflictEntry>,
) -> Option<ResolvedForbiddenGlobRequirements<Glob>>
where
    Glob: ForbiddenGlobRequirement,
    Glob::Identity: ToString,
{
    let globs = input
        .into_iter()
        .filter_map(|(prov, globs)| globs.map(|inner| (prov, inner)))
        .collect::<Vec<_>>();
    if globs.is_empty() {
        None
    } else {
        Some(resolve_forbidden_globs(key, globs, conflicts))
    }
}

impl EngineRequirement for CargoTomlRequirements {
    fn engine_id(&self) -> &'static str {
        crate::ENGINE_ID
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

fn resolve_item_map<K, Item>(
    input: Vec<(Provenance, BTreeMap<K, ItemRequirements<Item>>)>,
    key_path: impl Fn(&K) -> String,
    conflicts: &mut Vec<ConflictEntry>,
) -> BTreeMap<K, ResolvedItemRequirements<Item>>
where
    K: Ord + Clone,
    Item: FileItemRequirement,
    Item::Identity: ToString,
{
    let mut by_key: BTreeMap<K, Vec<(Provenance, ItemRequirements<Item>)>> = BTreeMap::new();
    for (prov, map) in input {
        for (key, items) in map {
            by_key.entry(key).or_default().push((prov.clone(), items));
        }
    }
    by_key
        .into_iter()
        .map(|(key, items)| {
            let path = key_path(&key);
            (key, resolve_items(&path, items, conflicts))
        })
        .collect()
}

fn resolve_maybe_items<Item>(
    key: &str,
    input: Vec<(Provenance, Option<ItemRequirements<Item>>)>,
    conflicts: &mut Vec<ConflictEntry>,
) -> Option<ResolvedItemRequirements<Item>>
where
    Item: FileItemRequirement,
    Item::Identity: ToString,
{
    let items = input
        .into_iter()
        .filter_map(|(prov, item_requirements)| item_requirements.map(|inner| (prov, inner)))
        .collect::<Vec<_>>();
    if items.is_empty() {
        None
    } else {
        Some(resolve_items(key, items, conflicts))
    }
}

fn resolve_profile_map(
    input: Vec<(Provenance, BTreeMap<String, ProfileRequirements>)>,
    conflicts: &mut Vec<ConflictEntry>,
) -> BTreeMap<String, ResolvedProfileRequirements> {
    let mut by_key: BTreeMap<String, Vec<(Provenance, ProfileRequirements)>> = BTreeMap::new();
    for (prov, map) in input {
        for (profile, req) in map {
            by_key.entry(profile).or_default().push((prov.clone(), req));
        }
    }
    by_key
        .into_iter()
        .map(|(profile, items)| {
            let key = format!("[profile.{profile}]");
            (
                profile,
                ProfileRequirements::resolve(&key, items, conflicts),
            )
        })
        .collect()
}

fn push_dependency_identity_overlaps(
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
    for (identity, requirement) in &merged.banned {
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

fn push_dependency_package_glob_conflicts(
    key: &str,
    merged: &ResolvedItemRequirements<DependencyRequirement>,
    globs: &ResolvedForbiddenGlobRequirements<DependencyPackageGlob>,
    conflicts: &mut Vec<ConflictEntry>,
) -> DependencyForbiddenGlobConflictBlocks {
    let mut blocks = DependencyForbiddenGlobConflictBlocks::default();
    for (glob_identity, glob) in &globs.globs {
        let Ok(compiled_glob) = GlobBuilder::new(&glob.merged.glob)
            .literal_separator(true)
            .build()
        else {
            continue;
        };
        let matcher = compiled_glob.compile_matcher();
        for (requirement_identity, requirement) in &merged.required {
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
            let _ = blocks.required.insert(requirement_identity.clone());
            let _ = blocks.package_globs.insert(glob_identity.clone());
        }
    }
    blocks
}

fn required_dependency_package(requirement: &DependencyRequirement) -> Option<String> {
    requirement
        .value
        .package
        .clone()
        .or_else(|| requirement.file_key.clone())
}
