//! Top-level reconcile dispatch for `CargoTomlEngine`. Calls into each
//! per-section submodule for each populated field of the requirement.

use aqc_file_engine_core::Finding;
use toml_edit::DocumentMut;

use super::{
    dependencies, features, lints, package_fields, package_lints, patch, profiles,
    section_presence, target_tables, workspace_fields,
};
use crate::requirement::CargoTomlRequirement;

/// Walk every section of `requirement`, applying its assertions to `doc` and
/// accumulating findings.
///
/// The requirement is destructured exhaustively (no `..`): a field added to
/// `CargoTomlRequirement` stops this function compiling until it is wired.
pub(crate) fn apply(
    doc: &mut DocumentMut,
    requirement: &CargoTomlRequirement,
    findings: &mut Vec<Finding>,
) {
    let CargoTomlRequirement {
        package_lints,
        workspace_lints,
        package_fields,
        workspace_package_fields,
        workspace_fields,
        section_presence,
        dependencies,
        workspace_dependencies,
        features,
        profiles,
        lib_fields,
        bin_targets,
        example_targets,
        test_targets,
        bench_targets,
        patch,
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
    dependencies::apply(doc, dependencies, findings);
    patch::apply_workspace_dependencies(doc, workspace_dependencies.as_ref(), findings);
    features::apply(doc, features.as_ref(), findings);
    profiles::apply(doc, profiles, findings);
    target_tables::apply_lib(doc, lib_fields, findings);
    target_tables::apply_named(doc, "bin", bin_targets, findings);
    target_tables::apply_named(doc, "example", example_targets, findings);
    target_tables::apply_named(doc, "test", test_targets, findings);
    target_tables::apply_named(doc, "bench", bench_targets, findings);
    patch::apply(doc, patch, findings);
}
