//! Forbidden `ignore` path-glob reconciliation.

use std::collections::BTreeSet;

use aqc_file_engine_core::{Finding, Provenance, ResolvedForbiddenGlobRequirements, Severity};
use globset::{GlobBuilder, GlobMatcher};
use toml_edit::DocumentMut;

use crate::requirement::RustfmtIgnorePathGlob;
use aqc_toml_engine_core::{list_values, report_list_shape_with_message, write_list};

/// Applies forbidden path globs to the `ignore` list.
pub(super) fn apply_forbidden_ignore_path_globs(
    doc: &mut DocumentMut,
    globs: &ResolvedForbiddenGlobRequirements<RustfmtIgnorePathGlob>,
    findings: &mut Vec<Finding>,
) {
    if globs.globs.is_empty() {
        return;
    }
    if report_ignore_glob_shape(doc, globs, findings) {
        let values = list_values(doc, "ignore");
        write_list(doc, "ignore", &values);
    }
    for entry in globs.globs.values() {
        let glob = &entry.merged;
        let attribution = entry.attribution();
        let message = entry
            .collected
            .first()
            .map(|(_, msg)| msg.clone())
            .unwrap_or_default();
        let matcher = match compile_ignore_path_glob(glob) {
            Ok(matcher) => matcher,
            Err(error_message) => {
                findings.push(Finding::InvalidRequirements {
                    key: format!("ignore.{}", glob.glob),
                    message: error_message,
                    contributors: entry
                        .collected
                        .iter()
                        .map(|(prov, msg)| (prov.policy.clone(), msg.clone()))
                        .collect(),
                });
                continue;
            }
        };
        remove_matching_ignore_values(doc, &matcher, &message, &attribution, findings);
    }
}

/// Removes `ignore` values matched by one compiled glob.
fn remove_matching_ignore_values(
    doc: &mut DocumentMut,
    matcher: &GlobMatcher,
    message: &str,
    attribution: &[Provenance],
    findings: &mut Vec<Finding>,
) {
    let values = list_values(doc, "ignore");
    let matching_values = values
        .iter()
        .filter(|value| matcher.is_match(value.as_str()))
        .cloned()
        .collect::<BTreeSet<_>>();
    if matching_values.is_empty() {
        return;
    }
    let kept = values
        .into_iter()
        .filter(|value| !matching_values.contains(value))
        .collect::<Vec<_>>();
    write_list(doc, "ignore", &kept);
    for path in matching_values {
        findings.push(Finding::Mismatch {
            key: format!("ignore.{path}"),
            selector: None,
            current: Some(path),
            expected: "absent (path glob)".to_owned(),
            message: message.to_owned(),
            severity: Severity::Error,
            attribution: attribution.to_vec(),
        });
    }
}

/// Reports malformed `ignore` list shape before glob matching.
fn report_ignore_glob_shape(
    doc: &DocumentMut,
    globs: &ResolvedForbiddenGlobRequirements<RustfmtIgnorePathGlob>,
    findings: &mut Vec<Finding>,
) -> bool {
    let attribution = ignore_glob_attribution(globs);
    report_list_shape_with_message(
        doc,
        "ignore",
        ignore_glob_message(globs),
        &attribution,
        findings,
    )
}

/// Returns the first message attached to forbidden ignore globs.
fn ignore_glob_message(globs: &ResolvedForbiddenGlobRequirements<RustfmtIgnorePathGlob>) -> String {
    globs
        .globs
        .values()
        .flat_map(|entry| entry.collected.iter().map(|(_, msg)| msg.as_str()))
        .next()
        .unwrap_or_default()
        .to_owned()
}

/// Returns attribution from all forbidden ignore glob contributors.
fn ignore_glob_attribution(
    globs: &ResolvedForbiddenGlobRequirements<RustfmtIgnorePathGlob>,
) -> Vec<Provenance> {
    globs
        .globs
        .values()
        .flat_map(aqc_file_engine_core::ResolvedRequirement::attribution)
        .collect()
}

/// Compiles a rustfmt ignore path glob.
fn compile_ignore_path_glob(glob: &RustfmtIgnorePathGlob) -> Result<GlobMatcher, String> {
    GlobBuilder::new(&glob.glob)
        .literal_separator(true)
        .backslash_escape(true)
        .build()
        .map(|glob| glob.compile_matcher())
        .map_err(|err| format!("invalid ignore path glob `{}`: {err}", glob.glob))
}
