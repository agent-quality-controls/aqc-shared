//! Validation and attribution helpers for `[toolchain]` reconciliation.

use std::collections::BTreeSet;
use std::path::Path;

use aqc_file_engine_core as file_core;
use aqc_file_engine_core::ScalarValue;
use aqc_toml_engine_core as toml_core;
use toml_edit::{Item, Table};

use crate::requirement::{
    ResolvedRustToolchainTomlRequirements, RustToolchainChannel, RustToolchainPath,
    RustToolchainProfile,
};

pub(super) fn report_invalid_requirement_combinations(
    requirement: &ResolvedRustToolchainTomlRequirements,
    findings: &mut Vec<file_core::Finding>,
) {
    if !has_path_value_requirement(requirement) {
        return;
    }
    if let Some(channel) = &requirement.channel {
        findings.push(invalid_requirements(
            "toolchain.channel",
            "`path` and `channel` cannot both be required.",
            requirement
                .path
                .iter()
                .flat_map(|resolved| scalar_contributors(&resolved.collected))
                .chain(scalar_contributors(&channel.collected))
                .collect(),
        ));
    }
    if let Some(profile) = &requirement.profile {
        findings.push(invalid_requirements(
            "toolchain.profile",
            "`path` disables channel-based toolchain fields.",
            requirement
                .path
                .iter()
                .flat_map(|resolved| scalar_contributors(&resolved.collected))
                .chain(scalar_contributors(&profile.collected))
                .collect(),
        ));
    }
    if !list_is_empty(&requirement.components) {
        findings.push(invalid_requirements(
            "toolchain.components",
            "`path` disables channel-based toolchain fields.",
            requirement
                .path
                .iter()
                .flat_map(|resolved| scalar_contributors(&resolved.collected))
                .chain(list_contributors(&requirement.components))
                .collect(),
        ));
    }
    if !list_is_empty(&requirement.targets) {
        findings.push(invalid_requirements(
            "toolchain.targets",
            "`path` disables channel-based toolchain fields.",
            requirement
                .path
                .iter()
                .flat_map(|resolved| scalar_contributors(&resolved.collected))
                .chain(list_contributors(&requirement.targets))
                .collect(),
        ));
    }
}

pub(super) fn report_existing_file_conflicts(
    table: &mut Table,
    requirement: &ResolvedRustToolchainTomlRequirements,
    findings: &mut Vec<file_core::Finding>,
) {
    if table.contains_key("channel") && table.contains_key("path") {
        push_unwritable(
            findings,
            "toolchain.path",
            "absent when channel is set",
            requirement,
        );
        let _ = table.remove("path");
    }
    if table.contains_key("path") && has_channel_based_requirements(requirement) {
        push_unwritable(
            findings,
            "toolchain.path",
            "absent when channel-based fields are required",
            requirement,
        );
        let _ = table.remove("path");
    }
}

pub(super) fn report_invalid_file_fields(
    table: &Table,
    requirement: &ResolvedRustToolchainTomlRequirements,
    findings: &mut Vec<file_core::Finding>,
) {
    report_string_file_field(table, "channel", requirement, findings);
    report_profile_file_field(table, requirement, findings);
    report_path_file_field(table, requirement, findings);
    report_list_file_field(table, "components", requirement, findings);
    report_list_file_field(table, "targets", requirement, findings);
}

fn report_string_file_field(
    table: &Table,
    key: &str,
    requirement: &ResolvedRustToolchainTomlRequirements,
    findings: &mut Vec<file_core::Finding>,
) {
    let Some(item) = table.get(key) else {
        return;
    };
    match item.as_str() {
        Some("") | None => push_unwritable(
            findings,
            format!("toolchain.{key}"),
            "non-empty string",
            requirement,
        ),
        Some(_) => {}
    }
}

fn report_profile_file_field(
    table: &Table,
    requirement: &ResolvedRustToolchainTomlRequirements,
    findings: &mut Vec<file_core::Finding>,
) {
    let Some(item) = table.get("profile") else {
        return;
    };
    let Some(value) = item.as_str() else {
        push_unwritable(findings, "toolchain.profile", "rustup profile", requirement);
        return;
    };
    if RustToolchainProfile::parse(value).is_err() {
        push_unwritable(findings, "toolchain.profile", "rustup profile", requirement);
    }
}

fn report_path_file_field(
    table: &Table,
    requirement: &ResolvedRustToolchainTomlRequirements,
    findings: &mut Vec<file_core::Finding>,
) {
    let Some(item) = table.get("path") else {
        return;
    };
    let Some(value) = item.as_str() else {
        push_unwritable(findings, "toolchain.path", "absolute path", requirement);
        return;
    };
    if value.is_empty() || !Path::new(value).is_absolute() {
        push_unwritable(findings, "toolchain.path", "absolute path", requirement);
    }
}

fn report_list_file_field(
    table: &Table,
    key: &str,
    requirement: &ResolvedRustToolchainTomlRequirements,
    findings: &mut Vec<file_core::Finding>,
) {
    let Some(item) = table.get(key) else {
        return;
    };
    let Some(array) = item.as_array() else {
        push_unwritable(
            findings,
            format!("toolchain.{key}"),
            "array of non-empty strings",
            requirement,
        );
        return;
    };
    let mut seen = BTreeSet::new();
    for value in array {
        let Some(value) = value.as_str() else {
            push_unwritable(
                findings,
                format!("toolchain.{key}"),
                "array of non-empty strings",
                requirement,
            );
            return;
        };
        if value.is_empty() || !seen.insert(value.to_owned()) {
            push_unwritable(
                findings,
                format!("toolchain.{key}"),
                "array of unique non-empty strings",
                requirement,
            );
            return;
        }
    }
}

pub(super) fn apply_closed(
    table: &mut Table,
    requirement: &ResolvedRustToolchainTomlRequirements,
    findings: &mut Vec<file_core::Finding>,
) {
    if requirement.exact_settings.is_empty() {
        return;
    }
    let allowed = ["channel", "path", "profile", "components", "targets"]
        .into_iter()
        .collect::<BTreeSet<_>>();
    let extras = table
        .iter()
        .map(|(key, _)| key.to_owned())
        .filter(|key| !allowed.contains(key.as_str()))
        .collect::<Vec<_>>();
    for extra in extras {
        findings.push(file_core::Finding::Mismatch {
            key: format!("toolchain.{extra}"),
            selector: None,
            current: table.get(&extra).and_then(toml_core::render_item),
            expected: "absent because rust-toolchain.toml fields are exact".to_owned(),
            message: requirement
                .exact_settings
                .first()
                .map(|(_, msg)| msg.clone())
                .unwrap_or_default(),
            severity: file_core::Severity::Error,
            attribution: requirement
                .exact_settings
                .iter()
                .map(|(prov, _)| prov.clone())
                .collect::<Vec<_>>(),
        });
        let _ = table.remove(&extra);
    }
}

pub(super) fn report_empty_table(table: &Table, findings: &mut Vec<file_core::Finding>) {
    if !table.is_empty() {
        return;
    }
    findings.push(file_core::Finding::Mismatch {
        key: "toolchain".to_owned(),
        selector: None,
        current: Some("{}".to_owned()),
        expected: "at least one supported property".to_owned(),
        message: "rust-toolchain.toml [toolchain] table cannot be empty.".to_owned(),
        severity: file_core::Severity::Error,
        attribution: Vec::new(),
    });
}

pub(super) fn scalar_attribution_for<T>(
    table: &Table,
    key: &str,
    resolved: &file_core::ResolvedRequirement<
        file_core::ScalarAssertion<T>,
        file_core::ScalarAssertion<T>,
    >,
    fails: impl Fn(Option<&Item>, &file_core::ScalarAssertion<T>) -> bool,
) -> Vec<file_core::Provenance> {
    let current = table.get(key);
    let filtered = resolved
        .collected
        .iter()
        .filter(|(_, assertion)| fails(current, assertion))
        .map(|(prov, _)| prov.clone())
        .collect::<Vec<_>>();
    if filtered.is_empty() {
        resolved.attribution()
    } else {
        filtered
    }
}

pub(super) fn channel_fails(
    current: Option<&Item>,
    assertion: &file_core::ScalarAssertion<RustToolchainChannel>,
) -> bool {
    string_assertion_fails(current, assertion, RustToolchainChannel::render)
}

pub(super) fn path_fails(
    current: Option<&Item>,
    assertion: &file_core::ScalarAssertion<RustToolchainPath>,
) -> bool {
    string_assertion_fails(current, assertion, RustToolchainPath::render)
}

pub(super) fn profile_fails(
    current: Option<&Item>,
    assertion: &file_core::ScalarAssertion<RustToolchainProfile>,
) -> bool {
    string_assertion_fails(current, assertion, RustToolchainProfile::render)
}

fn string_assertion_fails<T>(
    current: Option<&Item>,
    assertion: &file_core::ScalarAssertion<T>,
    render: impl Fn(&T) -> String,
) -> bool {
    match assertion {
        file_core::ScalarAssertion::Equals(want, _) => {
            current.and_then(Item::as_str) != Some(render(want).as_str())
        }
        file_core::ScalarAssertion::OneOf(allowed, _) => !current
            .and_then(Item::as_str)
            .is_some_and(|value| allowed.iter().map(&render).any(|allowed| allowed == value)),
        file_core::ScalarAssertion::Present(_) => current.is_none(),
        file_core::ScalarAssertion::Absent(_) => current.is_some(),
        file_core::ScalarAssertion::AtLeast(..)
        | file_core::ScalarAssertion::AtMost(..)
        | file_core::ScalarAssertion::Range(..) => true,
    }
}

pub(super) fn list_attribution(
    requirements: &file_core::ResolvedListRequirements,
) -> Vec<file_core::Provenance> {
    requirements
        .contains
        .values()
        .flat_map(file_core::ResolvedRequirement::attribution)
        .chain(
            requirements
                .excludes
                .values()
                .flat_map(file_core::ResolvedRequirement::attribution),
        )
        .chain(
            requirements
                .exact
                .iter()
                .flat_map(file_core::ResolvedRequirement::attribution),
        )
        .collect()
}

fn scalar_contributors<T>(
    collected: &[(file_core::Provenance, file_core::ScalarAssertion<T>)],
) -> Vec<(String, String)> {
    collected
        .iter()
        .map(|(prov, assertion)| (prov.policy.clone(), assertion.message().to_owned()))
        .collect()
}

fn list_contributors(requirements: &file_core::ResolvedListRequirements) -> Vec<(String, String)> {
    requirements
        .contains
        .values()
        .flat_map(|resolved| {
            resolved
                .collected
                .iter()
                .map(|(prov, msg)| (prov.policy.clone(), msg.clone()))
        })
        .chain(requirements.excludes.values().flat_map(|resolved| {
            resolved
                .collected
                .iter()
                .map(|(prov, msg)| (prov.policy.clone(), msg.clone()))
        }))
        .chain(requirements.exact.iter().flat_map(|resolved| {
            resolved
                .collected
                .iter()
                .map(|(prov, (_, msg))| (prov.policy.clone(), msg.clone()))
        }))
        .collect()
}

pub(super) fn requirement_attribution(
    requirement: &ResolvedRustToolchainTomlRequirements,
) -> Vec<file_core::Provenance> {
    requirement
        .channel
        .iter()
        .flat_map(file_core::ResolvedRequirement::attribution)
        .chain(
            requirement
                .path
                .iter()
                .flat_map(file_core::ResolvedRequirement::attribution),
        )
        .chain(
            requirement
                .profile
                .iter()
                .flat_map(file_core::ResolvedRequirement::attribution),
        )
        .chain(list_attribution(&requirement.components))
        .chain(list_attribution(&requirement.targets))
        .chain(
            requirement
                .exact_settings
                .iter()
                .map(|(prov, _)| prov.clone()),
        )
        .collect()
}

pub(super) fn has_requirements(requirement: &ResolvedRustToolchainTomlRequirements) -> bool {
    requirement.channel.is_some()
        || requirement.path.is_some()
        || requirement.profile.is_some()
        || !list_is_empty(&requirement.components)
        || !list_is_empty(&requirement.targets)
        || !requirement.exact_settings.is_empty()
}

pub(super) fn has_path_value_requirement(
    requirement: &ResolvedRustToolchainTomlRequirements,
) -> bool {
    requirement
        .path
        .as_ref()
        .is_some_and(|resolved| !matches!(resolved.merged, file_core::ScalarAssertion::Absent(_)))
}

fn has_channel_based_requirements(requirement: &ResolvedRustToolchainTomlRequirements) -> bool {
    requirement.channel.is_some()
        || requirement.profile.is_some()
        || !list_is_empty(&requirement.components)
        || !list_is_empty(&requirement.targets)
}

pub(super) fn list_is_empty(requirement: &file_core::ResolvedListRequirements) -> bool {
    requirement.contains.is_empty()
        && requirement.excludes.is_empty()
        && requirement.exact.is_none()
}

fn invalid_requirements(
    key: impl Into<String>,
    message: impl Into<String>,
    contributors: Vec<(String, String)>,
) -> file_core::Finding {
    file_core::Finding::InvalidRequirements {
        key: key.into(),
        message: message.into(),
        contributors,
    }
}

fn push_unwritable(
    findings: &mut Vec<file_core::Finding>,
    key: impl Into<String>,
    expected: impl Into<String>,
    requirement: &ResolvedRustToolchainTomlRequirements,
) {
    push_unwritable_with_attr(
        findings,
        key,
        expected,
        requirement_attribution(requirement),
    );
}

pub(super) fn push_unwritable_with_attr(
    findings: &mut Vec<file_core::Finding>,
    key: impl Into<String>,
    expected: impl Into<String>,
    attribution: Vec<file_core::Provenance>,
) {
    findings.push(file_core::Finding::UnwritableRequiredKey {
        key: key.into(),
        expected: expected.into(),
        attribution,
    });
}
