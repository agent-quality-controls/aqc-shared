use std::collections::{BTreeMap, BTreeSet};

use aqc_file_engine_core::{
    Finding, Provenance, ResolvedExactList, ResolvedForbiddenGlobRequirements,
    ResolvedListRequirements, Severity, apply_list_requirements, exact_list_difference,
};
use aqc_json_engine_core::{JsonObject, NonObjectParentAction};
use globset::{GlobBuilder, GlobMatcher};

use crate::types::{JsonPath, JsonStringGlob, ResolvedJsonFileRequirements};

pub(super) fn reconcile_string_list(
    document: &mut JsonObject,
    path: &JsonPath,
    requirement: &ResolvedListRequirements,
    forbidden_globs: &ResolvedForbiddenGlobRequirements<JsonStringGlob>,
    compiled_globs: &[CompiledGlob],
    findings: &mut Vec<Finding>,
) {
    let components = path.components().collect::<Vec<_>>();
    let exists = document.value_exists(&components);
    let blocked_parent = blocked_parent(document, &components);
    let current = document.string_list(&components);
    let valid_shape = !blocked_parent && (!exists || current.is_some());
    let current = current.unwrap_or_default();
    if !valid_shape {
        findings.push(Finding::Mismatch {
            key: path.finding_key(),
            selector: None,
            current: document.rendered_value(&components),
            expected: "string list".to_owned(),
            message: first_list_message(requirement, forbidden_globs),
            severity: Severity::Error,
            attribution: list_attribution(requirement, forbidden_globs),
        });
        return;
    }

    push_list_findings(path, &current, exists, requirement, findings);
    push_glob_findings(path, &current, compiled_globs, findings);

    let mut desired = apply_list_requirements(&current, requirement);
    desired.retain(|item| {
        !compiled_globs
            .iter()
            .any(|glob| glob.matcher.is_match(item))
    });
    if desired != current || (!exists && requirement.exact.is_some()) {
        let _ = document.set_string_list(&components, &desired, NonObjectParentAction::Preserve);
    }
}

fn blocked_parent(document: &JsonObject, path: &[&str]) -> bool {
    for depth in 1..path.len() {
        let Some(prefix) = path.get(..depth) else {
            continue;
        };
        if document.value_exists(prefix) && !document.object_exists(prefix) {
            return true;
        }
    }
    false
}

fn push_list_findings(
    path: &JsonPath,
    current: &[String],
    exists: bool,
    requirement: &ResolvedListRequirements,
    findings: &mut Vec<Finding>,
) {
    if let Some(exact) = &requirement.exact
        && !exists
    {
        findings.push(Finding::Mismatch {
            key: path.finding_key(),
            selector: None,
            current: None,
            expected: format!("exact [{}]", exact.merged.join(", ")),
            message: exact_message(exact),
            severity: Severity::Error,
            attribution: exact.attribution(),
        });
        return;
    }
    for (item, resolved) in &requirement.contains {
        if !current.contains(item) {
            push_item_finding(path, item, None, "present", resolved, findings);
        }
    }
    for (item, resolved) in &requirement.excludes {
        if current.contains(item) {
            push_item_finding(path, item, Some(item.clone()), "absent", resolved, findings);
        }
    }
    if let Some(exact) = &requirement.exact {
        push_exact_list_findings(path, current, exact, findings);
    }
}

fn push_exact_list_findings(
    path: &JsonPath,
    current: &[String],
    exact: &ResolvedExactList,
    findings: &mut Vec<Finding>,
) {
    let difference = exact_list_difference(current, &exact.merged);
    let message = exact_message(exact);
    for item in difference.missing().keys() {
        push_exact_member_finding(
            path,
            item,
            difference.current_count(item),
            difference.expected_count(item),
            &message,
            exact.attribution(),
            findings,
        );
    }
    for item in difference.unexpected().keys() {
        push_exact_member_finding(
            path,
            item,
            difference.current_count(item),
            difference.expected_count(item),
            &message,
            exact.attribution(),
            findings,
        );
    }
    if difference.order_mismatch() {
        findings.push(Finding::Mismatch {
            key: path.finding_key(),
            selector: None,
            current: Some(format!("[{}]", current.join(", "))),
            expected: format!("exact [{}]", exact.merged.join(", ")),
            message,
            severity: Severity::Error,
            attribution: exact.attribution(),
        });
    }
}

fn push_exact_member_finding(
    path: &JsonPath,
    item: &str,
    current_count: usize,
    expected_count: usize,
    message: &str,
    attribution: Vec<Provenance>,
    findings: &mut Vec<Finding>,
) {
    findings.push(Finding::Mismatch {
        key: path.finding_key(),
        selector: Some(item.to_owned()),
        current: Some(format!("count {current_count}")),
        expected: format!("count {expected_count}"),
        message: message.to_owned(),
        severity: Severity::Error,
        attribution,
    });
}

fn exact_message(exact: &ResolvedExactList) -> String {
    exact
        .collected
        .first()
        .map(|(_, (_, message))| message.clone())
        .unwrap_or_default()
}

fn push_item_finding(
    path: &JsonPath,
    item: &str,
    current: Option<String>,
    expected: &str,
    resolved: &aqc_file_engine_core::ResolvedRequirement<(), String>,
    findings: &mut Vec<Finding>,
) {
    findings.push(Finding::Mismatch {
        key: path.finding_key(),
        selector: Some(item.to_owned()),
        current,
        expected: expected.to_owned(),
        message: resolved
            .collected
            .first()
            .map(|(_, message)| message.clone())
            .unwrap_or_default(),
        severity: Severity::Error,
        attribution: resolved.attribution(),
    });
}

pub(super) struct CompiledGlob {
    matcher: GlobMatcher,
    message: String,
    attribution: Vec<Provenance>,
}

type CompiledGlobsByPath = BTreeMap<JsonPath, Vec<CompiledGlob>>;
type CompiledGlobResult = Result<Vec<CompiledGlob>, Vec<Finding>>;

pub(super) fn compile_all_globs(
    requirement: &ResolvedJsonFileRequirements,
    findings: &mut Vec<Finding>,
) -> Option<CompiledGlobsByPath> {
    let mut compiled = BTreeMap::new();
    let mut valid = true;
    for (path, globs) in requirement.forbidden_string_list_globs() {
        match compile_globs(path, globs) {
            Ok(path_globs) => {
                let _ = compiled.insert(path.clone(), path_globs);
            }
            Err(path_findings) => {
                valid = false;
                findings.extend(path_findings);
            }
        }
    }
    valid.then_some(compiled)
}

fn compile_globs(
    path: &JsonPath,
    requirements: &ResolvedForbiddenGlobRequirements<JsonStringGlob>,
) -> CompiledGlobResult {
    let mut compiled = Vec::new();
    let mut findings = Vec::new();
    for resolved in requirements.globs.values() {
        let glob = &resolved.merged.glob;
        let built = GlobBuilder::new(glob)
            .literal_separator(true)
            .backslash_escape(true)
            .build();
        match built {
            Ok(value) => compiled.push(CompiledGlob {
                matcher: value.compile_matcher(),
                message: resolved
                    .collected
                    .first()
                    .map(|(_, message)| message.clone())
                    .unwrap_or_default(),
                attribution: resolved.attribution(),
            }),
            Err(error) => {
                findings.push(Finding::InvalidRequirements {
                    key: path.finding_key(),
                    message: format!("invalid forbidden string-list glob `{glob}`: {error}"),
                    contributors: resolved
                        .collected
                        .iter()
                        .map(|(provenance, message)| (provenance.policy.clone(), message.clone()))
                        .collect(),
                });
            }
        }
    }
    if findings.is_empty() {
        Ok(compiled)
    } else {
        Err(findings)
    }
}

fn push_glob_findings(
    path: &JsonPath,
    current: &[String],
    globs: &[CompiledGlob],
    findings: &mut Vec<Finding>,
) {
    let mut seen = BTreeSet::new();
    for item in current {
        if !seen.insert(item) {
            continue;
        }
        let mut matches = globs.iter().filter(|glob| glob.matcher.is_match(item));
        let Some(first) = matches.next() else {
            continue;
        };
        let mut attribution = first.attribution.clone();
        for matched in matches {
            attribution.extend(matched.attribution.iter().cloned());
        }
        attribution.sort();
        attribution.dedup();
        findings.push(Finding::Mismatch {
            key: path.finding_key(),
            selector: Some(item.clone()),
            current: Some(item.clone()),
            expected: "absent (forbidden glob)".to_owned(),
            message: first.message.clone(),
            severity: Severity::Error,
            attribution,
        });
    }
}

fn first_list_message(
    requirement: &ResolvedListRequirements,
    globs: &ResolvedForbiddenGlobRequirements<JsonStringGlob>,
) -> String {
    requirement
        .contains
        .values()
        .chain(requirement.excludes.values())
        .flat_map(|resolved| resolved.collected.iter().map(|(_, message)| message))
        .chain(
            requirement
                .exact
                .iter()
                .flat_map(|resolved| resolved.collected.iter().map(|(_, (_, message))| message)),
        )
        .chain(
            globs
                .globs
                .values()
                .flat_map(|resolved| resolved.collected.iter().map(|(_, message)| message)),
        )
        .next()
        .cloned()
        .unwrap_or_default()
}

fn list_attribution(
    requirement: &ResolvedListRequirements,
    globs: &ResolvedForbiddenGlobRequirements<JsonStringGlob>,
) -> Vec<Provenance> {
    let mut attribution = requirement
        .contains
        .values()
        .chain(requirement.excludes.values())
        .flat_map(aqc_file_engine_core::ResolvedRequirement::attribution)
        .chain(
            requirement
                .exact
                .iter()
                .flat_map(aqc_file_engine_core::ResolvedRequirement::attribution),
        )
        .chain(
            globs
                .globs
                .values()
                .flat_map(aqc_file_engine_core::ResolvedRequirement::attribution),
        )
        .collect::<Vec<_>>();
    attribution.sort();
    attribution.dedup();
    attribution
}
