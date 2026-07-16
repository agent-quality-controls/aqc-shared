//! Reconcile resolved pnpm requirements against YAML bytes.

use std::collections::{BTreeMap, BTreeSet};

use crate::types::{PnpmPackageSelectorGlob, ResolvedPnpmWorkspaceYamlRequirements};
use aqc_file_engine_core::{
    Finding, KeyedItem, ResolvedForbiddenGlobRequirements, ResolvedItemRequirements,
    ResolvedListRequirements, Severity, apply_list_requirements, exact_list_difference,
};
use aqc_yaml_engine_core::{
    ParsedYamlMapping, YamlFieldValue, apply_scalar_assertion, parse_yaml_mapping,
};

use super::support;

pub(crate) fn reconcile(
    current_bytes: Option<&[u8]>,
    requirement: &ResolvedPnpmWorkspaceYamlRequirements,
) -> aqc_file_engine_core::EngineOutput {
    let document = match parse_yaml_mapping(current_bytes, "YAML document") {
        Ok(document) => document,
        Err(finding) => {
            return aqc_file_engine_core::EngineOutput {
                expected_bytes: current_bytes.unwrap_or_default().to_vec(),
                findings: vec![finding],
            };
        }
    };
    let mut findings = Vec::new();
    apply_scalars(&document, requirement, &mut findings);
    apply_list(
        &document,
        "minimumReleaseAgeExclude",
        &requirement.minimum_release_age_exclude,
        &requirement.forbidden_minimum_release_age_exclude_globs,
        &mut findings,
    );
    apply_list(
        &document,
        "trustPolicyExclude",
        &requirement.trust_policy_exclude,
        &requirement.forbidden_trust_policy_exclude_globs,
        &mut findings,
    );
    apply_allow_builds(&document, requirement, &mut findings);
    apply_exact_settings(&document, requirement, &mut findings);
    aqc_file_engine_core::EngineOutput {
        expected_bytes: document.render(),
        findings,
    }
}

fn apply_scalars(
    document: &ParsedYamlMapping,
    requirement: &ResolvedPnpmWorkspaceYamlRequirements,
    findings: &mut Vec<Finding>,
) {
    apply_scalar_assertion(
        document,
        "strictPeerDependencies",
        requirement.strict_peer_dependencies.as_ref(),
        findings,
    );
    apply_scalar_assertion(
        document,
        "engineStrict",
        requirement.engine_strict.as_ref(),
        findings,
    );
    apply_scalar_assertion(
        document,
        "minimumReleaseAge",
        requirement.minimum_release_age.as_ref(),
        findings,
    );
    apply_scalar_assertion(
        document,
        "minimumReleaseAgeStrict",
        requirement.minimum_release_age_strict.as_ref(),
        findings,
    );
    apply_scalar_assertion(
        document,
        "minimumReleaseAgeIgnoreMissingTime",
        requirement.minimum_release_age_ignore_missing_time.as_ref(),
        findings,
    );
    apply_scalar_assertion(
        document,
        "trustPolicy",
        requirement.trust_policy.as_ref(),
        findings,
    );
    apply_scalar_assertion(
        document,
        "trustLockfile",
        requirement.trust_lockfile.as_ref(),
        findings,
    );
    apply_scalar_assertion(
        document,
        "trustPolicyIgnoreAfter",
        requirement.trust_policy_ignore_after.as_ref(),
        findings,
    );
    apply_scalar_assertion(
        document,
        "blockExoticSubdeps",
        requirement.block_exotic_subdeps.as_ref(),
        findings,
    );
    apply_scalar_assertion(
        document,
        "pmOnFail",
        requirement.pm_on_fail.as_ref(),
        findings,
    );
    apply_scalar_assertion(
        document,
        "strictDepBuilds",
        requirement.strict_dep_builds.as_ref(),
        findings,
    );
    apply_scalar_assertion(
        document,
        "dangerouslyAllowAllBuilds",
        requirement.dangerously_allow_all_builds.as_ref(),
        findings,
    );
}

fn apply_list(
    document: &ParsedYamlMapping,
    key: &str,
    requirement: &ResolvedListRequirements,
    forbidden_globs: &ResolvedForbiddenGlobRequirements<PnpmPackageSelectorGlob>,
    findings: &mut Vec<Finding>,
) {
    if !has_list_requirement(requirement) && forbidden_globs.globs.is_empty() {
        return;
    }
    let compiled_globs = support::compile_globs(key, forbidden_globs, findings);
    let field = document.field(key);
    let (current, valid_shape, exists) = match &field {
        Ok(Some(YamlFieldValue::StringSequence(values))) => (values.clone(), true, true),
        Ok(None) => (Vec::new(), true, false),
        Ok(Some(_)) | Err(_) => {
            support::push_shape_finding(
                key,
                support::list_attribution(requirement, forbidden_globs),
                findings,
            );
            (Vec::new(), false, true)
        }
    };
    if valid_shape {
        push_list_findings(key, &current, exists, requirement, findings);
        support::push_forbidden_selector_findings(key, &current, &compiled_globs, findings);
    }
    let mut desired = apply_list_requirements(&current, requirement);
    desired.retain(|item| {
        !compiled_globs
            .iter()
            .any(|glob| glob.matcher.is_match(item))
    });
    if desired != current || !valid_shape || (!exists && requirement.exact.is_some()) {
        document.set_string_sequence(key, &desired);
    }
}

fn push_list_findings(
    key: &str,
    current: &[String],
    exists: bool,
    requirement: &ResolvedListRequirements,
    findings: &mut Vec<Finding>,
) {
    if !exists {
        if let Some(exact) = &requirement.exact {
            findings.push(Finding::Mismatch {
                key: key.to_owned(),
                selector: None,
                current: None,
                expected: format!("{:?}", exact.merged),
                message: exact_message(exact),
                severity: Severity::Error,
                attribution: exact.attribution(),
            });
            return;
        }
    }
    for (item, resolved) in &requirement.contains {
        if !current.contains(item) {
            push_list_member_finding(
                key,
                item,
                None,
                "present".to_owned(),
                first_member_message(resolved),
                resolved.attribution(),
                findings,
            );
        }
    }
    for (item, resolved) in &requirement.excludes {
        if current.contains(item) {
            push_list_member_finding(
                key,
                item,
                Some(item.clone()),
                "absent".to_owned(),
                first_member_message(resolved),
                resolved.attribution(),
                findings,
            );
        }
    }
    let Some(exact) = &requirement.exact else {
        return;
    };
    let difference = exact_list_difference(current, &exact.merged);
    let message = exact_message(exact);
    for item in difference.missing().keys() {
        push_list_member_finding(
            key,
            item,
            Some(format!("count {}", difference.current_count(item))),
            format!("count {}", difference.expected_count(item)),
            message.clone(),
            exact.attribution(),
            findings,
        );
    }
    for item in difference.unexpected().keys() {
        push_list_member_finding(
            key,
            item,
            Some(format!("count {}", difference.current_count(item))),
            format!("count {}", difference.expected_count(item)),
            message.clone(),
            exact.attribution(),
            findings,
        );
    }
    if difference.order_mismatch() {
        findings.push(Finding::Mismatch {
            key: key.to_owned(),
            selector: None,
            current: Some(format!("{current:?}")),
            expected: format!("{:?}", exact.merged),
            message,
            severity: Severity::Error,
            attribution: exact.attribution(),
        });
    }
}

#[allow(clippy::too_many_arguments)] // reason: finding construction keeps every reported field explicit.
fn push_list_member_finding(
    key: &str,
    item: &str,
    current: Option<String>,
    expected: String,
    message: String,
    attribution: Vec<aqc_file_engine_core::Provenance>,
    findings: &mut Vec<Finding>,
) {
    findings.push(Finding::Mismatch {
        key: key.to_owned(),
        selector: Some(item.to_owned()),
        current,
        expected,
        message,
        severity: Severity::Error,
        attribution,
    });
}

fn first_member_message(
    resolved: &aqc_file_engine_core::ResolvedRequirement<(), String>,
) -> String {
    resolved
        .collected
        .first()
        .map_or_else(String::new, |(_, message)| message.clone())
}

fn exact_message(
    exact: &aqc_file_engine_core::ResolvedRequirement<Vec<String>, (Vec<String>, String)>,
) -> String {
    exact
        .collected
        .first()
        .map_or_else(String::new, |(_, (_, message))| message.clone())
}

fn apply_allow_builds(
    document: &ParsedYamlMapping,
    requirement: &ResolvedPnpmWorkspaceYamlRequirements,
    findings: &mut Vec<Finding>,
) {
    if requirement.allow_builds.required.is_empty()
        && requirement.allow_builds.forbidden.is_empty()
        && requirement.allow_builds.exact.is_none()
        && requirement
            .forbidden_allowed_build_package_globs
            .globs
            .is_empty()
    {
        return;
    }
    let compiled_globs = support::compile_globs(
        "allowBuilds",
        &requirement.forbidden_allowed_build_package_globs,
        findings,
    );
    let field = document.field("allowBuilds");
    let (current, valid_shape) = match &field {
        Ok(Some(YamlFieldValue::StringBooleanMapping(values))) => (values.clone(), true),
        Ok(None) => (BTreeMap::new(), true),
        Ok(Some(_)) | Err(_) => {
            support::push_shape_finding(
                "allowBuilds",
                support::item_attribution(
                    &requirement.allow_builds,
                    &requirement.forbidden_allowed_build_package_globs,
                ),
                findings,
            );
            (BTreeMap::new(), false)
        }
    };
    for (selector, allowed) in &current {
        if *allowed {
            support::push_glob_item_finding("allowBuilds", selector, &compiled_globs, findings);
        }
    }
    let mut desired = desired_items(&current, &requirement.allow_builds);
    let requirement_changed = desired != current;
    desired.retain(|selector, allowed| {
        !*allowed
            || !compiled_globs
                .iter()
                .any(|glob| glob.matcher.is_match(selector))
    });
    if requirement_changed {
        support::push_collection_mismatch(
            "allowBuilds",
            "allowBuilds entries do not satisfy the resolved requirement".to_owned(),
            support::item_attribution(
                &requirement.allow_builds,
                &requirement.forbidden_allowed_build_package_globs,
            ),
            None,
            findings,
        );
    }
    if desired != current || !valid_shape {
        document.set_string_boolean_mapping("allowBuilds", &desired);
    }
}

fn desired_items(
    current: &BTreeMap<String, bool>,
    requirement: &ResolvedItemRequirements<KeyedItem<bool>>,
) -> BTreeMap<String, bool> {
    let mut desired = requirement.exact.as_ref().map_or_else(
        || current.clone(),
        |exact| {
            exact
                .items
                .values()
                .map(|item| (item.merged.file_key.clone(), item.merged.value))
                .collect()
        },
    );
    for item in requirement.required.values() {
        let _ = desired.insert(item.merged.file_key.clone(), item.merged.value);
    }
    for key in requirement.forbidden.keys() {
        let _ = desired.remove(key);
    }
    desired
}

fn apply_exact_settings(
    document: &ParsedYamlMapping,
    requirement: &ResolvedPnpmWorkspaceYamlRequirements,
    findings: &mut Vec<Finding>,
) {
    if requirement.exact_settings.is_empty() {
        return;
    }
    let attribution = requirement
        .exact_settings
        .iter()
        .map(|(provenance, _)| provenance.clone())
        .collect::<Vec<_>>();
    let message = requirement
        .exact_settings
        .first()
        .map_or_else(String::new, |(_, message)| message.clone());
    let Ok(effective) = document.effective_keys() else {
        support::push_collection_mismatch("<<", message, attribution, None, findings);
        return;
    };
    let direct = document.direct_keys().into_iter().collect::<BTreeSet<_>>();
    let authorized = authorized_settings(requirement);
    for key in effective
        .into_iter()
        .filter(|key| !authorized.contains(key.as_str()))
    {
        findings.push(Finding::Mismatch {
            key: key.clone(),
            selector: None,
            current: Some("present".to_owned()),
            expected: "setting absent".to_owned(),
            message: message.clone(),
            severity: Severity::Error,
            attribution: attribution.clone(),
        });
        if direct.contains(&key) {
            let rendered = document.render();
            let candidate = parse_yaml_mapping(Some(&rendered), "YAML document");
            if candidate.is_ok_and(|candidate| {
                candidate.remove(&key);
                candidate
                    .effective_keys()
                    .is_ok_and(|keys| !keys.contains(&key))
            }) {
                document.remove(&key);
            }
        }
    }
}

fn authorized_settings(requirement: &ResolvedPnpmWorkspaceYamlRequirements) -> BTreeSet<&str> {
    let mut settings = BTreeSet::new();
    for (key, present) in [
        (
            "strictPeerDependencies",
            requirement.strict_peer_dependencies.is_some(),
        ),
        ("engineStrict", requirement.engine_strict.is_some()),
        (
            "minimumReleaseAge",
            requirement.minimum_release_age.is_some(),
        ),
        (
            "minimumReleaseAgeStrict",
            requirement.minimum_release_age_strict.is_some(),
        ),
        (
            "minimumReleaseAgeIgnoreMissingTime",
            requirement
                .minimum_release_age_ignore_missing_time
                .is_some(),
        ),
        ("trustPolicy", requirement.trust_policy.is_some()),
        ("trustLockfile", requirement.trust_lockfile.is_some()),
        (
            "trustPolicyIgnoreAfter",
            requirement.trust_policy_ignore_after.is_some(),
        ),
        (
            "blockExoticSubdeps",
            requirement.block_exotic_subdeps.is_some(),
        ),
        ("pmOnFail", requirement.pm_on_fail.is_some()),
        ("strictDepBuilds", requirement.strict_dep_builds.is_some()),
        (
            "dangerouslyAllowAllBuilds",
            requirement.dangerously_allow_all_builds.is_some(),
        ),
    ] {
        if present {
            let _ = settings.insert(key);
        }
    }
    if has_list_requirement(&requirement.minimum_release_age_exclude)
        || !requirement
            .forbidden_minimum_release_age_exclude_globs
            .globs
            .is_empty()
    {
        let _ = settings.insert("minimumReleaseAgeExclude");
    }
    if has_list_requirement(&requirement.trust_policy_exclude)
        || !requirement
            .forbidden_trust_policy_exclude_globs
            .globs
            .is_empty()
    {
        let _ = settings.insert("trustPolicyExclude");
    }
    if !requirement.allow_builds.required.is_empty()
        || !requirement.allow_builds.forbidden.is_empty()
        || requirement.allow_builds.exact.is_some()
        || !requirement
            .forbidden_allowed_build_package_globs
            .globs
            .is_empty()
    {
        let _ = settings.insert("allowBuilds");
    }
    settings
}

fn has_list_requirement(requirement: &ResolvedListRequirements) -> bool {
    !requirement.contains.is_empty()
        || !requirement.excludes.is_empty()
        || requirement.exact.is_some()
}
