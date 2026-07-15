use std::cmp::Reverse;
use std::collections::{BTreeMap, BTreeSet};

use aqc_file_engine_core::{
    Finding, KeyedItem, Provenance, ResolvedForbiddenGlobRequirements, ResolvedItemRequirements,
    ResolvedListRequirements, Severity, asserted_items,
};
use aqc_json_engine_core::{JsonObject, NonObjectParentAction, reconcile_scalar_assertion};
use globset::{GlobBuilder, GlobMatcher};

use crate::types::{JsonPath, JsonStringGlob, ResolvedJsonFileRequirements};

pub(super) fn reconcile_document(
    document: &mut JsonObject,
    requirement: &ResolvedJsonFileRequirements,
    findings: &mut Vec<Finding>,
) {
    let Some(compiled_globs) = compile_all_globs(requirement, findings) else {
        return;
    };
    for (path, assertion) in requirement.scalar_values() {
        let components = path.components().collect::<Vec<_>>();
        reconcile_scalar_assertion(
            document,
            &components,
            path.finding_key(),
            Some(path.selector()),
            NonObjectParentAction::Preserve,
            assertion,
            |value| Some(value.clone()),
            Some,
            findings,
        );
    }

    let paths = requirement
        .string_lists()
        .keys()
        .chain(requirement.forbidden_string_list_globs().keys())
        .cloned()
        .collect::<BTreeSet<_>>();
    for path in paths {
        let lists = requirement
            .string_lists()
            .get(&path)
            .cloned()
            .unwrap_or_default();
        let globs = requirement
            .forbidden_string_list_globs()
            .get(&path)
            .cloned()
            .unwrap_or_default();
        let compiled = compiled_globs.get(&path).map_or(&[][..], Vec::as_slice);
        reconcile_string_list(document, &path, &lists, &globs, compiled, findings);
    }
    let mut objects = requirement.object_keys().iter().collect::<Vec<_>>();
    objects.sort_by_key(|(path, _)| Reverse(path.components().count()));
    for (path, keys) in objects {
        reconcile_object_keys(document, path, keys, findings);
    }
}

fn reconcile_string_list(
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

    let mut desired = requirement
        .exact
        .as_ref()
        .map_or_else(|| current.clone(), |exact| exact.merged.clone());
    for item in requirement.contains.keys() {
        if !desired.contains(item) {
            desired.push(item.clone());
        }
    }
    desired.retain(|item| !requirement.excludes.contains_key(item));
    desired.retain(|item| {
        !compiled_globs
            .iter()
            .any(|glob| glob.matcher.is_match(item))
    });
    if desired != current || (!exists && requirement.exact.is_some()) {
        let _ = document.set_string_list(&components, &desired, NonObjectParentAction::Preserve);
    }
}

fn reconcile_object_keys(
    document: &mut JsonObject,
    path: &JsonPath,
    requirement: &ResolvedItemRequirements<KeyedItem<()>>,
    findings: &mut Vec<Finding>,
) {
    let components = path.components().collect::<Vec<_>>();
    let exists = components.is_empty() || document.value_exists(&components);
    if !exists {
        let constructive = requirement.exact.is_some() || !requirement.required.is_empty();
        if !constructive {
            return;
        }
        findings.push(Finding::Mismatch {
            key: path.finding_key(),
            selector: None,
            current: None,
            expected: "object".to_owned(),
            message: object_message(requirement),
            severity: Severity::Error,
            attribution: object_attribution(requirement),
        });
        if !document.set_object(&components, NonObjectParentAction::Preserve) {
            return;
        }
    }
    let Some(current) = document.object_keys(&components) else {
        findings.push(Finding::Mismatch {
            key: path.finding_key(),
            selector: None,
            current: document.rendered_value(&components),
            expected: "object".to_owned(),
            message: object_message(requirement),
            severity: Severity::Error,
            attribution: object_attribution(requirement),
        });
        return;
    };
    let current = current.into_iter().collect::<BTreeSet<_>>();
    for (key, resolved) in asserted_items(requirement) {
        if !current.contains(key) {
            findings.push(Finding::UnwritableRequiredKey {
                key: child_finding_key(path, key),
                expected: "present object key".to_owned(),
                attribution: resolved.attribution(),
            });
        }
    }
    for (key, resolved) in &requirement.forbidden {
        if current.contains(key) {
            push_object_key_finding(
                path,
                key,
                "absent",
                resolved
                    .collected
                    .first()
                    .map_or_else(String::new, |(_, message)| message.clone()),
                resolved.attribution(),
                findings,
            );
            let _ = document.remove_object_key(&components, key);
        }
    }
    if let Some(exact) = &requirement.exact {
        for key in current.difference(&exact.identities) {
            push_object_key_finding(
                path,
                key,
                "absent (exact keys)",
                exact
                    .collected
                    .first()
                    .map_or_else(String::new, |(_, (_, message))| message.clone()),
                exact_attribution(exact),
                findings,
            );
            let _ = document.remove_object_key(&components, key);
        }
    }
}

fn push_object_key_finding(
    path: &JsonPath,
    key: &str,
    expected: &str,
    message: String,
    attribution: Vec<Provenance>,
    findings: &mut Vec<Finding>,
) {
    findings.push(Finding::Mismatch {
        key: path.finding_key(),
        selector: Some(key.to_owned()),
        current: Some("present".to_owned()),
        expected: expected.to_owned(),
        message,
        severity: Severity::Error,
        attribution,
    });
}

fn object_message(requirement: &ResolvedItemRequirements<KeyedItem<()>>) -> String {
    requirement
        .required
        .values()
        .flat_map(|resolved| resolved.collected.iter().map(|(_, (_, message))| message))
        .chain(
            requirement
                .forbidden
                .values()
                .flat_map(|resolved| resolved.collected.iter().map(|(_, message)| message)),
        )
        .chain(
            requirement
                .exact
                .iter()
                .flat_map(|resolved| resolved.collected.iter().map(|(_, (_, message))| message)),
        )
        .next()
        .cloned()
        .unwrap_or_default()
}

fn object_attribution(requirement: &ResolvedItemRequirements<KeyedItem<()>>) -> Vec<Provenance> {
    let mut attribution = requirement
        .required
        .values()
        .flat_map(aqc_file_engine_core::ResolvedRequirement::attribution)
        .chain(
            requirement
                .forbidden
                .values()
                .flat_map(aqc_file_engine_core::ResolvedRequirement::attribution),
        )
        .chain(requirement.exact.iter().flat_map(exact_attribution))
        .collect::<Vec<_>>();
    attribution.sort();
    attribution.dedup();
    attribution
}

fn exact_attribution(
    exact: &aqc_file_engine_core::ResolvedExactItems<KeyedItem<()>>,
) -> Vec<Provenance> {
    exact
        .collected
        .iter()
        .map(|(provenance, _)| provenance.clone())
        .collect()
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

fn escape_pointer(component: &str) -> String {
    component.replace('~', "~0").replace('/', "~1")
}

fn child_finding_key(path: &JsonPath, child: &str) -> String {
    if path == &JsonPath::root() {
        format!("/{}", escape_pointer(child))
    } else {
        format!("{}/{}", path.pointer(), escape_pointer(child))
    }
}

fn push_list_findings(
    path: &JsonPath,
    current: &[String],
    exists: bool,
    requirement: &ResolvedListRequirements,
    findings: &mut Vec<Finding>,
) {
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
    if let Some(exact) = &requirement.exact
        && (!exists || current != exact.merged)
    {
        findings.push(Finding::Mismatch {
            key: path.finding_key(),
            selector: None,
            current: Some(format!("[{}]", current.join(", "))),
            expected: format!("exact [{}]", exact.merged.join(", ")),
            message: exact
                .collected
                .first()
                .map(|(_, (_, message))| message.clone())
                .unwrap_or_default(),
            severity: Severity::Error,
            attribution: exact.attribution(),
        });
    }
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

struct CompiledGlob {
    matcher: GlobMatcher,
    message: String,
    attribution: Vec<Provenance>,
}

type CompiledGlobsByPath = BTreeMap<JsonPath, Vec<CompiledGlob>>;
type CompiledGlobResult = Result<Vec<CompiledGlob>, Vec<Finding>>;

fn compile_all_globs(
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
