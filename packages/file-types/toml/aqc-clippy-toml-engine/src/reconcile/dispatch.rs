//! Top-level reconcile dispatch for `ClippyTomlEngine`.

use aqc_file_engine_core::Finding;
use toml_edit::DocumentMut;

use super::{bans, bools, enums, msrv, thresholds};
use crate::requirement::ClippyTomlRequirement;

/// Walk every non-empty section of `requirement`, applying its assertions
/// to `doc` and accumulating findings.
pub(crate) fn apply(
    doc: &mut DocumentMut,
    requirement: &ClippyTomlRequirement,
    findings: &mut Vec<Finding>,
) {
    if let Some(m) = requirement.msrv.as_ref() {
        msrv::apply(doc, m, findings);
    }
    if let Some(m) = requirement.thresholds.as_ref() {
        thresholds::apply(doc, m, findings);
    }
    if let Some(m) = requirement.disallowed_methods.as_ref() {
        bans::apply(doc, "disallowed-methods", m, findings);
    }
    if let Some(m) = requirement.disallowed_types.as_ref() {
        bans::apply(doc, "disallowed-types", m, findings);
    }
    if let Some(m) = requirement.disallowed_macros.as_ref() {
        bans::apply(doc, "disallowed-macros", m, findings);
    }
    bools::apply(doc, &requirement.bools, findings);
    enums::apply(doc, &requirement.enums, findings);
}
