//! Top-level reconcile dispatch for `ClippyTomlEngine`.

use aqc_file_engine_core::Finding;
use toml_edit::DocumentMut;

use super::{bools, disallowed, enums, msrv, thresholds};
use crate::requirement::ResolvedClippyTomlRequirements;

/// Walk every non-empty section of `requirement`, applying its assertions
/// to `doc` and accumulating findings.
pub(crate) fn apply(
    doc: &mut DocumentMut,
    requirement: &ResolvedClippyTomlRequirements,
    findings: &mut Vec<Finding>,
) {
    if let Some(m) = requirement.msrv.as_ref() {
        msrv::apply(doc, m, findings);
    }
    thresholds::apply(doc, &requirement.thresholds, findings);
    disallowed::apply(
        doc,
        "disallowed-methods",
        &requirement.disallowed_methods,
        &requirement.forbidden_disallowed_method_path_globs,
        &requirement.disallowed_method_glob_conflicts,
        findings,
    );
    disallowed::apply(
        doc,
        "disallowed-types",
        &requirement.disallowed_types,
        &requirement.forbidden_disallowed_type_path_globs,
        &requirement.disallowed_type_glob_conflicts,
        findings,
    );
    disallowed::apply(
        doc,
        "disallowed-macros",
        &requirement.disallowed_macros,
        &requirement.forbidden_disallowed_macro_path_globs,
        &requirement.disallowed_macro_glob_conflicts,
        findings,
    );
    bools::apply(doc, &requirement.bools, findings);
    enums::apply(doc, &requirement.enums, findings);
}
