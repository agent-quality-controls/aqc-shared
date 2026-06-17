//! Reconcile `[patch.<registry>]` tables. Reuses the dependency helpers
//! against the nested `[patch.<registry>]` table.

use std::collections::BTreeMap;

use aqc_file_engine_core::{Finding, ResolvedItemRequirements, ResolvedPatternBanRequirements};
use toml_edit::DocumentMut;

use crate::reconcile::dependencies::{SetRule, apply_set};
use crate::requirement::{
    DependencyPackagePattern, DependencyPatternConflictBlocks, DependencyRequirement,
};

/// Apply every `[patch.<registry>]` requirement.
pub(crate) fn apply(
    doc: &mut DocumentMut,
    merged_by_registry: &BTreeMap<String, ResolvedItemRequirements<DependencyRequirement>>,
    banned_patch_dependency_package_patterns: &BTreeMap<
        String,
        ResolvedPatternBanRequirements<DependencyPackagePattern>,
    >,
    patch_dependency_pattern_conflicts: &BTreeMap<String, DependencyPatternConflictBlocks>,
    findings: &mut Vec<Finding>,
) {
    let empty_items = ResolvedItemRequirements::default();
    let empty_patterns = ResolvedPatternBanRequirements::default();
    let empty_conflicts = DependencyPatternConflictBlocks::default();
    let registries = merged_by_registry
        .keys()
        .chain(banned_patch_dependency_package_patterns.keys())
        .chain(patch_dependency_pattern_conflicts.keys())
        .collect::<std::collections::BTreeSet<_>>();
    for registry in registries {
        let path = vec!["patch".to_owned(), registry.clone()];
        let display = format!("[patch.{registry}]");
        let merged = merged_by_registry.get(registry).unwrap_or(&empty_items);
        let patterns = banned_patch_dependency_package_patterns
            .get(registry)
            .unwrap_or(&empty_patterns);
        let pattern_conflicts = patch_dependency_pattern_conflicts
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
            patterns,
            pattern_conflicts,
            findings,
        );
    }
}

/// Apply the single `[workspace.dependencies]` requirement.
pub(crate) fn apply_workspace_dependencies(
    doc: &mut DocumentMut,
    merged: Option<&ResolvedItemRequirements<DependencyRequirement>>,
    banned_workspace_dependency_package_patterns: Option<
        &ResolvedPatternBanRequirements<DependencyPackagePattern>,
    >,
    workspace_dependency_pattern_conflicts: &DependencyPatternConflictBlocks,
    findings: &mut Vec<Finding>,
) {
    if merged.is_none()
        && banned_workspace_dependency_package_patterns.is_none()
        && workspace_dependency_pattern_conflicts.required.is_empty()
        && workspace_dependency_pattern_conflicts
            .package_patterns
            .is_empty()
    {
        return;
    }
    let empty_items = ResolvedItemRequirements::default();
    let empty_patterns = ResolvedPatternBanRequirements::default();
    let path = vec!["workspace".to_owned(), "dependencies".to_owned()];
    apply_set(
        doc,
        &path,
        "[workspace.dependencies]",
        SetRule::WorkspaceDeps,
        merged.unwrap_or(&empty_items),
        banned_workspace_dependency_package_patterns.unwrap_or(&empty_patterns),
        workspace_dependency_pattern_conflicts,
        findings,
    );
}
