//! Reconcile `[patch.<registry>]` tables. Reuses the dependency helpers
//! against the nested `[patch.<registry>]` table.

#![expect(
    clippy::type_complexity,
    reason = "Patch reconciliation consumes resolved dependency and forbidden-glob maps."
)]

use std::collections::BTreeMap;

use aqc_file_engine_core::{Finding, ResolvedForbiddenGlobRequirements, ResolvedItemRequirements};
use toml_edit::DocumentMut;

use crate::reconcile::dependencies::{SetRule, apply_set};
use crate::requirement::{
    DependencyForbiddenGlobConflictBlocks, DependencyPackageGlob, DependencyRequirement,
};

/// Apply every `[patch.<registry>]` requirement.
pub(crate) fn apply(
    doc: &mut DocumentMut,
    merged_by_registry: &BTreeMap<String, ResolvedItemRequirements<DependencyRequirement>>,
    forbidden_patch_dependency_package_globs: &BTreeMap<
        String,
        ResolvedForbiddenGlobRequirements<DependencyPackageGlob>,
    >,
    patch_dependency_glob_conflicts: &BTreeMap<String, DependencyForbiddenGlobConflictBlocks>,
    findings: &mut Vec<Finding>,
) {
    let empty_items = ResolvedItemRequirements::default();
    let empty_globs = ResolvedForbiddenGlobRequirements::default();
    let empty_conflicts = DependencyForbiddenGlobConflictBlocks::default();
    let registries = merged_by_registry
        .keys()
        .chain(forbidden_patch_dependency_package_globs.keys())
        .chain(patch_dependency_glob_conflicts.keys())
        .collect::<std::collections::BTreeSet<_>>();
    for registry in registries {
        let path = vec!["patch".to_owned(), registry.clone()];
        let display = format!("[patch.{registry}]");
        let merged = merged_by_registry.get(registry).unwrap_or(&empty_items);
        let globs = forbidden_patch_dependency_package_globs
            .get(registry)
            .unwrap_or(&empty_globs);
        let glob_conflicts = patch_dependency_glob_conflicts
            .get(registry)
            .unwrap_or(&empty_conflicts);
        // Package-only patch entries are not writable; the shared helper emits
        // UnwritableRequiredKey when it cannot use a `file_key`.
        apply_set(
            doc,
            &path,
            &display,
            SetRule::Patch,
            merged,
            globs,
            glob_conflicts,
            findings,
        );
    }
}

/// Apply the single `[workspace.dependencies]` requirement.
pub(crate) fn apply_workspace_dependencies(
    doc: &mut DocumentMut,
    merged: Option<&ResolvedItemRequirements<DependencyRequirement>>,
    forbidden_workspace_dependency_package_globs: Option<
        &ResolvedForbiddenGlobRequirements<DependencyPackageGlob>,
    >,
    workspace_dependency_glob_conflicts: &DependencyForbiddenGlobConflictBlocks,
    findings: &mut Vec<Finding>,
) {
    if merged.is_none()
        && forbidden_workspace_dependency_package_globs.is_none()
        && workspace_dependency_glob_conflicts.required.is_empty()
        && workspace_dependency_glob_conflicts.package_globs.is_empty()
    {
        return;
    }
    let empty_items = ResolvedItemRequirements::default();
    let empty_globs = ResolvedForbiddenGlobRequirements::default();
    let path = vec!["workspace".to_owned(), "dependencies".to_owned()];
    apply_set(
        doc,
        &path,
        "[workspace.dependencies]",
        SetRule::WorkspaceDeps,
        merged.unwrap_or(&empty_items),
        forbidden_workspace_dependency_package_globs.unwrap_or(&empty_globs),
        workspace_dependency_glob_conflicts,
        findings,
    );
}
