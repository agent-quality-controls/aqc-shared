//! Top-level reconcile dispatch for `CargoTomlEngine`. Calls into each
//! per-section submodule for each populated field of the requirement.

use aqc_file_engine_core::Finding;
use toml_edit::DocumentMut;

use super::{
    dependencies, features, lints, package_fields, package_lints, patch, profiles,
    section_presence, target_tables, workspace_fields,
};
use crate::requirement::ResolvedCargoTomlRequirements;

/// Walk every section of `requirement`, applying its assertions to `doc` and
/// accumulating findings.
///
/// The requirement is destructured exhaustively (no `..`): a field added to
/// `ResolvedCargoTomlRequirements` stops this function compiling until it is wired.
pub(crate) fn apply(
    doc: &mut DocumentMut,
    requirement: &ResolvedCargoTomlRequirements,
    findings: &mut Vec<Finding>,
) {
    let ResolvedCargoTomlRequirements {
        package_lints,
        workspace_lints,
        package_fields,
        workspace_package_fields,
        workspace_fields,
        section_presence,
        dependencies,
        banned_dependency_package_patterns,
        dependency_pattern_conflicts,
        workspace_dependencies,
        banned_workspace_dependency_package_patterns,
        workspace_dependency_pattern_conflicts,
        features,
        profiles,
        targets,
        patch,
        banned_patch_dependency_package_patterns,
        patch_dependency_pattern_conflicts,
    } = requirement;

    package_lints::apply(doc, package_lints.as_ref(), findings);
    lints::apply(doc, lints::LintRoot::Workspace, workspace_lints, findings);
    package_fields::apply(
        doc,
        package_fields::PackageScope::Package,
        package_fields,
        findings,
    );
    package_fields::apply(
        doc,
        package_fields::PackageScope::WorkspacePackage,
        workspace_package_fields,
        findings,
    );
    workspace_fields::apply(doc, workspace_fields, findings);
    section_presence::apply(doc, section_presence, findings);
    dependencies::apply(
        doc,
        dependencies,
        banned_dependency_package_patterns,
        dependency_pattern_conflicts,
        findings,
    );
    patch::apply_workspace_dependencies(
        doc,
        workspace_dependencies.as_ref(),
        banned_workspace_dependency_package_patterns.as_ref(),
        workspace_dependency_pattern_conflicts,
        findings,
    );
    features::apply(doc, features.as_ref(), findings);
    profiles::apply(doc, profiles, findings);
    target_tables::apply_lib(doc, &targets.lib_fields, findings);
    target_tables::apply_named(doc, "bin", &targets.bin_targets, findings);
    target_tables::apply_named(doc, "example", &targets.example_targets, findings);
    target_tables::apply_named(doc, "test", &targets.test_targets, findings);
    target_tables::apply_named(doc, "bench", &targets.bench_targets, findings);
    patch::apply(
        doc,
        patch,
        banned_patch_dependency_package_patterns,
        patch_dependency_pattern_conflicts,
        findings,
    );
}
