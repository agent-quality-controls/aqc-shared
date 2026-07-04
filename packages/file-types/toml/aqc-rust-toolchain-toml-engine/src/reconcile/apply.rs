//! Reconcile entrypoint for Rust toolchain TOML documents.

use aqc_file_engine_core::Finding;
use toml_edit::DocumentMut;

use crate::reconcile::settings;
use crate::requirement::ResolvedRustToolchainTomlRequirements;

pub(crate) fn apply(
    doc: &mut DocumentMut,
    requirement: &ResolvedRustToolchainTomlRequirements,
    findings: &mut Vec<Finding>,
) {
    settings::apply(doc, requirement, findings);
}
