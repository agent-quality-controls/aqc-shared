//! Top-level reconcile dispatch for `CargoTomlEngine`. Calls into each
//! per-section submodule for each populated field of the requirement.

use aqc_file_engine_core::{Finding, Severity};
use toml_edit::DocumentMut;

use super::{
    dependencies, features, lints, lints_inherit, package_fields, patch, profiles,
    section_presence, target_tables, workspace_fields,
};
use crate::requirement::{CargoTomlRequirement, LintsInheritAssertion};

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
        lints,
        workspace_lints,
        lints_inherit,
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

    // Exclusivity rule: cargo rejects `[lints] workspace = true` combined with
    // inline `[lints.<tool>]` tables in the same manifest. Never write that
    // manifest; surface the incompatible requirement set instead.
    if inherit_asserted(lints_inherit.as_ref()) && !lints.is_empty() {
        findings.push(Finding::SchemaError {
            path: "[lints].workspace".to_owned(),
            message: "incompatible requirement set: cargo rejects `[lints] workspace = true` \
                      combined with inline `[lints.<tool>]` tables in the same manifest; drop \
                      one side (workspace lint tables `[workspace.lints.<tool>]` plus the \
                      opt-in is the standard pattern)"
                .to_owned(),
            severity: Severity::Error,
        });
    } else {
        lints::apply(doc, lints::LintRoot::Package, lints, findings);
        lints_inherit::apply(doc, lints_inherit.as_ref(), findings);
    }
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

/// True when the requirement asserts the member opt-in is (or must be) set:
/// `Equals(true)` or `Present`. `Equals(false)` / `Absent` do not collide
/// with inline lint tables.
fn inherit_asserted(
    merged: Option<&aqc_file_engine_core::MergedAssertion<LintsInheritAssertion>>,
) -> bool {
    merged.is_some_and(|m| {
        m.contributions.iter().any(|(_, a)| {
            matches!(
                a,
                LintsInheritAssertion::Equals(true, _) | LintsInheritAssertion::Present(_)
            )
        })
    })
}
