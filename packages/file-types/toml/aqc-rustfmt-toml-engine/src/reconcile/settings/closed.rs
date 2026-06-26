//! Closed-settings reconciliation.

use std::collections::BTreeSet;

use aqc_file_engine_core::{Finding, Severity};
use toml_edit::DocumentMut;

use crate::requirement::ResolvedRustfmtTomlRequirements;
use aqc_toml_engine_core::render_item;

/// Removes settings not named by a closed requirement set.
pub(super) fn apply_closed(
    doc: &mut DocumentMut,
    requirement: &ResolvedRustfmtTomlRequirements,
    findings: &mut Vec<Finding>,
) {
    if requirement.closed_settings.is_empty() {
        return;
    }
    let allowed = requirement
        .scalar_settings
        .keys()
        .map(|key| key.file_key())
        .chain(requirement.list_settings.keys().map(|key| key.file_key()))
        .chain((!requirement.forbidden_ignore_path_globs.globs.is_empty()).then_some("ignore"))
        .collect::<BTreeSet<_>>();
    let extras = doc
        .as_table()
        .iter()
        .map(|(key, _)| key.to_owned())
        .filter(|key| !allowed.contains(key.as_str()))
        .collect::<Vec<_>>();
    for extra in extras {
        findings.push(Finding::Mismatch {
            key: extra.clone(),
            current: doc.get(&extra).and_then(render_item),
            expected: "absent because rustfmt.toml settings are closed".to_owned(),
            message: requirement
                .closed_settings
                .first()
                .map(|(_, msg)| msg.clone())
                .unwrap_or_default(),
            severity: Severity::Error,
            attribution: requirement
                .closed_settings
                .iter()
                .map(|(prov, _)| prov.clone())
                .collect(),
        });
        let _ = doc.as_table_mut().remove(&extra);
    }
}
