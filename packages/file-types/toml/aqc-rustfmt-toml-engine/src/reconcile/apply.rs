//! Reconcile entrypoint for Rustfmt TOML documents.

use aqc_file_engine_core::Finding;
use toml_edit::DocumentMut;

use crate::reconcile::settings;
use crate::requirement::ResolvedRustfmtTomlRequirements;

/// Applies resolved rustfmt requirements to a TOML document.
pub(crate) fn apply(
    doc: &mut DocumentMut,
    requirement: &ResolvedRustfmtTomlRequirements,
    findings: &mut Vec<Finding>,
) {
    settings::apply(doc, requirement, findings);
}
