//! Exact deny.toml settings reconciliation.

use std::collections::BTreeSet;

use aqc_file_engine_core::{Finding, Provenance};
use aqc_toml_engine_core::push_mismatch;
use toml_edit::DocumentMut;

use crate::requirement::ResolvedDenyTomlRequirements;

use super::support::table_path_mut;

pub(super) fn apply_exact_settings(
    doc: &mut DocumentMut,
    requirement: &ResolvedDenyTomlRequirements,
    findings: &mut Vec<Finding>,
) {
    if requirement.exact_settings.is_empty() {
        return;
    }
    let allowed_root = BTreeSet::from([
        "graph",
        "output",
        "advisories",
        "licenses",
        "bans",
        "sources",
    ]);
    let attribution = requirement
        .exact_settings
        .iter()
        .map(|(prov, _)| prov.clone())
        .collect::<Vec<_>>();
    let message = requirement
        .exact_settings
        .first()
        .map(|(_, msg)| msg.clone())
        .unwrap_or_default();
    let extras = doc
        .iter()
        .filter_map(|(key, _)| (!allowed_root.contains(key)).then_some(key.to_owned()))
        .collect::<Vec<_>>();
    for key in extras {
        let current = doc.get(&key).map(ToString::to_string);
        let _ = doc.remove(&key);
        push_mismatch(
            findings,
            key,
            current,
            "absent (exact settings)".to_owned(),
            message.clone(),
            &attribution,
        );
    }
    exact_table(
        doc,
        &["bans", "build"],
        &[
            "executables",
            "interpreted",
            "script-extensions",
            "enable-builtin-globs",
            "globs",
            "include-dependencies",
            "include-workspace",
            "include-archives",
        ],
        &message,
        &attribution,
        findings,
    );
}

fn exact_table(
    doc: &mut DocumentMut,
    table_path: &[&str],
    allowed: &[&str],
    message: &str,
    attribution: &[Provenance],
    findings: &mut Vec<Finding>,
) {
    let Some(table) = table_path_mut(doc, table_path) else {
        return;
    };
    let allowed = allowed.iter().copied().collect::<BTreeSet<_>>();
    let extras = table
        .iter()
        .filter_map(|(key, _)| (!allowed.contains(key)).then_some(key.to_owned()))
        .collect::<Vec<_>>();
    for key in extras {
        let current = table.get(&key).map(ToString::to_string);
        let _ = table.remove(&key);
        push_mismatch(
            findings,
            format!("{}.{}", table_path.join("."), key),
            current,
            "absent (exact settings)".to_owned(),
            message.to_owned(),
            attribution,
        );
    }
}
