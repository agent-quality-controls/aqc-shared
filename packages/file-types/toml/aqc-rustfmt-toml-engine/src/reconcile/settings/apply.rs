//! Settings reconciliation dispatch.

use aqc_file_engine_core::{Finding, Provenance, ResolvedRequirement};
use toml_edit::{DocumentMut, Item};

use super::closed::apply_closed;
use super::ignore::apply_forbidden_ignore_path_globs;
use super::list::apply_list;
use super::scalar::{apply_scalar, scalar_matches};
use super::toml_io::attribution;
use crate::requirement::{
    ResolvedRustfmtScalarAssertion, ResolvedRustfmtTomlRequirements, RustfmtScalarAssertion,
};

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
    resolved: &ResolvedRequirement<ResolvedRustfmtScalarAssertion, RustfmtScalarAssertion>,
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
fn scalar_assertion_fails(current: Option<&Item>, assertion: &RustfmtScalarAssertion) -> bool {
    match assertion {
        RustfmtScalarAssertion::Equals(want, _) => {
            !current.is_some_and(|item| scalar_matches(item, want))
        }
        RustfmtScalarAssertion::OneOf(allowed, _) => !current
            .and_then(Item::as_str)
            .is_some_and(|value| allowed.contains(value)),
        RustfmtScalarAssertion::Present(_) => current.is_none(),
        RustfmtScalarAssertion::Absent(_) => current.is_some(),
    }
}
