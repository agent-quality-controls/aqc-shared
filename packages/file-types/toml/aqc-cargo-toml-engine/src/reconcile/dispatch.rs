//! Top-level reconcile dispatch for `CargoTomlEngine`. Calls into each
//! per-section submodule for each populated field of the requirement.

use aqc_file_engine_core::Finding;
use toml_edit::DocumentMut;

use super::{
    dependencies, features, lints, lints_inherit, package_fields, profiles, workspace_lints,
    workspace_package_fields,
};
use crate::requirement::CargoTomlRequirement;

/// Walk every non-empty section of `requirement`, applying its assertions
/// to `doc` and accumulating findings.
pub(crate) fn apply(
    doc: &mut DocumentMut,
    requirement: &CargoTomlRequirement,
    findings: &mut Vec<Finding>,
) {
    lints::apply(doc, &requirement.lints, findings);
    lints_inherit::apply(doc, requirement.lints_inherit.as_ref(), findings);
    workspace_lints::apply(doc, &requirement.workspace_lints, findings);
    package_fields::apply(doc, &requirement.package_fields, findings);
    workspace_package_fields::apply(doc, &requirement.workspace_package_fields, findings);
    profiles::apply(doc, &requirement.profiles, findings);
    dependencies::apply(doc, &requirement.dependencies, findings);
    features::apply(doc, requirement.features.as_ref(), findings);
}
