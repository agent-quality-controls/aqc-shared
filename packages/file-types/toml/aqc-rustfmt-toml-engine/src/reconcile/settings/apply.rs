//! Settings reconciliation dispatch.

use aqc_file_engine_core::{
    ConfigScalar, Finding, Provenance, ResolvedRequirement, ScalarAssertion,
};
use toml_edit::{DocumentMut, Item};

use super::closed::apply_closed;
use super::ignore::apply_forbidden_ignore_path_globs;
use super::list::apply_list;
use super::scalar::{apply_scalar, scalar_matches};
use super::toml_io::attribution;
use crate::requirement::ResolvedRustfmtTomlRequirements;

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
    apply_forbidden_ignore_path_globs(
        doc,
        &requirement.forbidden_ignore_path_globs,
        &requirement.ignore_glob_conflicts.path_globs,
        findings,
    );
    apply_closed(doc, requirement, findings);
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

/// Returns whether a raw scalar assertion is unsatisfied by the current TOML item.
fn scalar_assertion_fails(
    current: Option<&Item>,
    assertion: &ScalarAssertion<ConfigScalar>,
) -> bool {
    match assertion {
        ScalarAssertion::Equals(want, _) => !current.is_some_and(|item| scalar_matches(item, want)),
        ScalarAssertion::OneOf(allowed, _) => {
            !current.is_some_and(|item| allowed.iter().any(|want| scalar_matches(item, want)))
        }
        ScalarAssertion::Present(_) => current.is_none(),
        ScalarAssertion::Absent(_) => current.is_some(),
        ScalarAssertion::AtLeast(..) | ScalarAssertion::AtMost(..) | ScalarAssertion::Range(..) => {
            true
        }
    }
}
