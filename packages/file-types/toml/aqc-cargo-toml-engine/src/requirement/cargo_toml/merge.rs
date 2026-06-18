//! Requirement aggregate merge implementation.

use std::collections::BTreeMap;

use aqc_file_engine_core::{ConflictEntry, Provenance, resolve_map};

use super::super::dependencies::{DependencyPackageGlob, DependencyRequirement, DependencyScope};
use super::super::lints::PackageLintsAssertion;
use super::super::sections::ManifestSection;
use super::super::targets::TargetRequirements;
use super::conflicts::{push_dependency_identity_overlaps, push_dependency_package_glob_conflicts};
use super::model::{
    CargoTomlRequirements, DependencyForbiddenGlobConflictBlocks, ResolvedCargoTomlRequirements,
};
use super::resolve::{
    resolve_forbidden_glob_map, resolve_item_map, resolve_maybe_forbidden_globs,
    resolve_maybe_items, resolve_profile_map,
};

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
            |scope| scope.table_path(),
            &mut conflicts,
        );
        for (scope, merged) in &dependencies {
            push_dependency_identity_overlaps(&scope.table_path(), merged, &mut conflicts);
        }

        let forbidden_dependency_package_globs = resolve_forbidden_glob_map(
            reqs.iter()
                .map(|(prov, req)| (prov.clone(), req.forbidden_dependency_package_globs.clone()))
                .collect(),
            |scope| scope.table_path(),
            &mut conflicts,
        );
        let dependency_glob_conflicts = collect_dependency_glob_conflicts(
            &dependencies,
            &forbidden_dependency_package_globs,
            &mut conflicts,
        );

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
        let patch_dependency_glob_conflicts = collect_patch_glob_conflicts(
            &patch,
            &forbidden_patch_dependency_package_globs,
            &mut conflicts,
        );

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
        let workspace_dependency_glob_conflicts = collect_workspace_glob_conflicts(
            &workspace_dependencies,
            &forbidden_workspace_dependency_package_globs,
            &mut conflicts,
        );

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

fn collect_dependency_glob_conflicts(
    dependencies: &BTreeMap<
        DependencyScope,
        aqc_file_engine_core::ResolvedItemRequirements<DependencyRequirement>,
    >,
    forbidden_globs: &BTreeMap<
        DependencyScope,
        aqc_file_engine_core::ResolvedForbiddenGlobRequirements<DependencyPackageGlob>,
    >,
    conflicts: &mut Vec<ConflictEntry>,
) -> BTreeMap<DependencyScope, DependencyForbiddenGlobConflictBlocks> {
    let mut blocks_by_key = BTreeMap::new();
    for (scope, globs) in forbidden_globs {
        let Some(merged) = dependencies.get(scope) else {
            continue;
        };
        let blocks =
            push_dependency_package_glob_conflicts(&scope.table_path(), merged, globs, conflicts);
        if !blocks.is_empty() {
            let _ = blocks_by_key.insert(scope.clone(), blocks);
        }
    }
    blocks_by_key
}

fn collect_patch_glob_conflicts(
    dependencies: &BTreeMap<
        String,
        aqc_file_engine_core::ResolvedItemRequirements<DependencyRequirement>,
    >,
    forbidden_globs: &BTreeMap<
        String,
        aqc_file_engine_core::ResolvedForbiddenGlobRequirements<DependencyPackageGlob>,
    >,
    conflicts: &mut Vec<ConflictEntry>,
) -> BTreeMap<String, DependencyForbiddenGlobConflictBlocks> {
    let mut blocks_by_key = BTreeMap::new();
    for (registry, globs) in forbidden_globs {
        let Some(merged) = dependencies.get(registry) else {
            continue;
        };
        let blocks = push_dependency_package_glob_conflicts(
            &format!("[patch.{registry}]"),
            merged,
            globs,
            conflicts,
        );
        if !blocks.is_empty() {
            let _ = blocks_by_key.insert(registry.clone(), blocks);
        }
    }
    blocks_by_key
}

fn collect_workspace_glob_conflicts(
    dependencies: &Option<aqc_file_engine_core::ResolvedItemRequirements<DependencyRequirement>>,
    forbidden_globs: &Option<
        aqc_file_engine_core::ResolvedForbiddenGlobRequirements<DependencyPackageGlob>,
    >,
    conflicts: &mut Vec<ConflictEntry>,
) -> DependencyForbiddenGlobConflictBlocks {
    let Some(merged) = dependencies else {
        return DependencyForbiddenGlobConflictBlocks::default();
    };
    let Some(globs) = forbidden_globs else {
        return DependencyForbiddenGlobConflictBlocks::default();
    };
    push_dependency_package_glob_conflicts("[workspace.dependencies]", merged, globs, conflicts)
}
