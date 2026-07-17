use std::cmp::Reverse;
use std::collections::BTreeSet;

use aqc_file_engine_core::{
    FileItemRequirement, Finding, KeyedItem, Provenance, ResolvedItemRequirements, Severity,
    item_presence_difference,
};
use aqc_json_engine_core::{JsonObject, NonObjectParentAction, reconcile_scalar_assertion};

use crate::types::{JsonPath, ResolvedJsonFileRequirements};

use super::lists::{compile_all_globs, reconcile_string_list};

pub(in crate::runtime) fn reconcile_document(
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
    let difference = item_presence_difference(&current, requirement);
    for (key, resolved) in difference.missing {
        findings.push(Finding::UnwritableRequiredKey {
            key: child_finding_key(path, key),
            expected: "present object key".to_owned(),
            attribution: resolved.attribution(),
        });
    }
    for (key, resolved) in difference.forbidden {
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
    if let Some(membership) = requirement.membership() {
        for key in difference.unexpected {
            push_object_key_finding(
                path,
                key,
                if membership.is_exact() {
                    "absent (exact keys)"
                } else {
                    "absent (not allowed)"
                },
                membership
                    .message_for_rejected(|item| item.merge_identity() == *key)
                    .to_owned(),
                membership.attribution_for_rejected(|item| item.merge_identity() == *key),
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
                .allowed
                .iter()
                .flat_map(|resolved| resolved.collected.iter().map(|(_, (_, message))| message)),
        )
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
        .chain(requirement.allowed.iter().flat_map(|allowed| {
            allowed
                .collected
                .iter()
                .map(|(provenance, _)| provenance.clone())
        }))
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
