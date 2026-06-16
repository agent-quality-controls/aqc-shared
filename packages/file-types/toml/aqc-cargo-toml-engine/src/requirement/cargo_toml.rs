//! `Cargo.toml` requirement aggregate and merge phase.

#![expect(
    clippy::disallowed_types,
    reason = "`Any` is used only for EngineRequirement downcast dispatch."
)]

use core::any::Any;
use std::collections::{BTreeMap, BTreeSet};

use aqc_file_engine_core::{
    ConflictEntry, EngineRequirement, FileItemRequirement, ItemRequirements, KeyedItem, Provenance,
    ResolvedItemRequirements, ResolvedRequirement, resolve_items, resolve_map,
};

use super::dependencies::{DependencyIdentity, DependencyRequirement, DependencyScope};
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
    pub workspace_dependencies: Option<ItemRequirements<DependencyRequirement>>,
    pub features: Option<ItemRequirements<KeyedItem<FeatureMembers>>>,
    pub profiles: BTreeMap<String, ProfileRequirements>,
    pub targets: TargetRequirements,
    pub patch: BTreeMap<String, ItemRequirements<DependencyRequirement>>,
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
    pub workspace_dependencies: Option<ResolvedItemRequirements<DependencyRequirement>>,
    pub features: Option<ResolvedItemRequirements<KeyedItem<FeatureMembers>>>,
    pub profiles: BTreeMap<String, ResolvedProfileRequirements>,
    pub targets: ResolvedTargetRequirements,
    pub patch: BTreeMap<String, ResolvedItemRequirements<DependencyRequirement>>,
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
            workspace_dependencies,
            features,
            profiles,
            targets,
            patch,
        };

        (out, conflicts)
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
