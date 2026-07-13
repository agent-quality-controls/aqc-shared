//! Settings reconciliation dispatch.

use aqc_file_engine_core::{
    ConfigScalar, Finding, Provenance, ResolvedRequirement, ScalarAssertion,
};
use toml_edit::DocumentMut;

use super::exact::apply_exact;
use super::ignore::apply_forbidden_ignore_path_globs;
use super::list::apply_list;
use super::scalar::apply_scalar;
use crate::requirement::ResolvedRustfmtTomlRequirements;
use aqc_toml_engine_core::{attribution, scalar_assertion_fails};

/// Resolved rustfmt scalar-setting assertion.
type ResolvedRustfmtScalarSetting =
    ResolvedRequirement<ScalarAssertion<ConfigScalar>, ScalarAssertion<ConfigScalar>>;

/// Applies every resolved rustfmt setting group.
pub(crate) fn apply(
    doc: &mut DocumentMut,
    requirement: &ResolvedRustfmtTomlRequirements,
    findings: &mut Vec<Finding>,
) {
    for (setting, resolved) in &requirement.scalar_settings {
        let key = setting.file_key();
        let attribution = scalar_attribution_for(doc, key, resolved);
        apply_scalar(doc, key, &resolved.merged, &attribution, findings);
    }
    for (setting, resolved) in &requirement.list_settings {
        apply_list(doc, setting.file_key(), resolved, findings);
    }
    apply_forbidden_ignore_path_globs(doc, &requirement.forbidden_ignore_path_globs, findings);
    apply_exact(doc, requirement, findings);
}

/// Keeps attribution only from scalar assertions that fail the current value.
fn scalar_attribution_for(
    doc: &DocumentMut,
    key: &str,
    resolved: &ResolvedRustfmtScalarSetting,
) -> Vec<Provenance> {
    let current = doc.get(key);
    let filtered = resolved
        .collected
        .iter()
        .filter(|(_, assertion)| scalar_assertion_fails(current, assertion))
        .map(|(prov, _)| prov.clone())
        .collect::<Vec<_>>();
    if filtered.is_empty() {
        attribution(resolved)
    } else {
        filtered
    }
}
