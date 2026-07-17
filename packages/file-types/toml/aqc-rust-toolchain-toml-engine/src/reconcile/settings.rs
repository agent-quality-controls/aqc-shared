//! Reconcile `[toolchain]` fields.

use aqc_file_engine_core as file_core;
use aqc_file_engine_core::ScalarValue;
use aqc_toml_engine_core as toml_core;
use toml_edit::{DocumentMut, Item, Table, value as toml_value};

use super::settings_support as support;
use crate::requirement::{
    ResolvedRustToolchainTomlRequirements, RustToolchainChannel, RustToolchainPath,
    RustToolchainProfile,
};

type ResolvedChannel = file_core::ResolvedRequirement<
    file_core::ScalarAssertion<RustToolchainChannel>,
    file_core::ScalarAssertion<RustToolchainChannel>,
>;
type ResolvedPath = file_core::ResolvedRequirement<
    file_core::ScalarAssertion<RustToolchainPath>,
    file_core::ScalarAssertion<RustToolchainPath>,
>;
type ResolvedProfile = file_core::ResolvedRequirement<
    file_core::ScalarAssertion<RustToolchainProfile>,
    file_core::ScalarAssertion<RustToolchainProfile>,
>;

pub(crate) fn apply(
    doc: &mut DocumentMut,
    requirement: &ResolvedRustToolchainTomlRequirements,
    findings: &mut Vec<file_core::Finding>,
) {
    support::report_invalid_requirement_combinations(requirement, findings);
    if doc.get("toolchain").is_none() && !support::requires_toolchain_table(requirement) {
        return;
    }
    let table = ensure_toolchain_table(doc, requirement, findings);
    toml_core::remove_rejected_table_keys(
        table,
        "toolchain",
        &requirement.toolchain_keys,
        findings,
    );
    support::report_invalid_file_fields(table, requirement, findings);
    support::report_existing_file_conflicts(table, requirement, findings);

    if support::has_path_value_requirement(requirement) {
        if let Some(path) = &requirement.path {
            apply_path(table, path, findings);
        }
        if let Some(channel) = requirement
            .channel
            .as_ref()
            .filter(|resolved| matches!(resolved.merged, file_core::ScalarAssertion::Absent(_)))
        {
            apply_channel(table, channel, findings);
        }
        if let Some(profile) = requirement
            .profile
            .as_ref()
            .filter(|resolved| matches!(resolved.merged, file_core::ScalarAssertion::Absent(_)))
        {
            apply_profile(table, profile, findings);
        }
        if !support::list_requires_presence(&requirement.components) {
            apply_list(table, "components", &requirement.components, findings);
        }
        if !support::list_requires_presence(&requirement.targets) {
            apply_list(table, "targets", &requirement.targets, findings);
        }
        toml_core::report_missing_table_keys(
            table,
            "toolchain",
            &requirement.toolchain_keys,
            findings,
        );
        support::report_empty_table(table, findings);
        return;
    }

    if let Some(path) = &requirement.path {
        apply_path(table, path, findings);
    }
    if let Some(channel) = &requirement.channel {
        apply_channel(table, channel, findings);
    }
    if let Some(profile) = &requirement.profile {
        apply_profile(table, profile, findings);
    }
    apply_list(table, "components", &requirement.components, findings);
    apply_list(table, "targets", &requirement.targets, findings);
    toml_core::report_missing_table_keys(table, "toolchain", &requirement.toolchain_keys, findings);
    support::report_empty_table(table, findings);
}

fn ensure_toolchain_table<'a>(
    doc: &'a mut DocumentMut,
    requirement: &ResolvedRustToolchainTomlRequirements,
    findings: &mut Vec<file_core::Finding>,
) -> &'a mut Table {
    if doc.get("toolchain").and_then(Item::as_table).is_none() {
        if doc.get("toolchain").is_some() {
            support::push_unwritable_with_attr(
                findings,
                "toolchain",
                "table",
                support::requirement_attribution(requirement),
            );
            let _ = doc.remove("toolchain");
        } else if support::requires_toolchain_table(requirement) {
            findings.push(file_core::Finding::Mismatch {
                key: "toolchain".to_owned(),
                selector: None,
                current: None,
                expected: "table".to_owned(),
                message: "rust-toolchain.toml must contain a [toolchain] table.".to_owned(),
                severity: file_core::Severity::Error,
                attribution: support::requirement_attribution(requirement),
            });
        }
    }
    toml_core::ensure_table(doc, "toolchain")
}

fn apply_channel(
    table: &mut Table,
    resolved: &ResolvedChannel,
    findings: &mut Vec<file_core::Finding>,
) {
    apply_string_scalar(
        table,
        "channel",
        &resolved.merged,
        &support::scalar_attribution_for(table, "channel", resolved, support::channel_fails),
        RustToolchainChannel::render,
        findings,
    );
}

fn apply_path(table: &mut Table, resolved: &ResolvedPath, findings: &mut Vec<file_core::Finding>) {
    apply_string_scalar(
        table,
        "path",
        &resolved.merged,
        &support::scalar_attribution_for(table, "path", resolved, support::path_fails),
        RustToolchainPath::render,
        findings,
    );
}

fn apply_profile(
    table: &mut Table,
    resolved: &ResolvedProfile,
    findings: &mut Vec<file_core::Finding>,
) {
    apply_string_scalar(
        table,
        "profile",
        &resolved.merged,
        &support::scalar_attribution_for(table, "profile", resolved, support::profile_fails),
        RustToolchainProfile::render,
        findings,
    );
}

fn apply_string_scalar<T>(
    table: &mut Table,
    key: &str,
    assertion: &file_core::ScalarAssertion<T>,
    attribution: &[file_core::Provenance],
    render: impl Fn(&T) -> String,
    findings: &mut Vec<file_core::Finding>,
) {
    let current = table.get(key);
    let display_key = format!("toolchain.{key}");
    match assertion {
        file_core::ScalarAssertion::Equals(want, message) => {
            let expected = render(want);
            if current.and_then(Item::as_str) == Some(expected.as_str()) {
                return;
            }
            findings.push(file_core::Finding::Mismatch {
                key: display_key,
                selector: None,
                current: current.and_then(toml_core::render_item),
                expected: expected.clone(),
                message: message.clone(),
                severity: file_core::Severity::Error,
                attribution: attribution.to_vec(),
            });
            table[key] = toml_value(expected);
        }
        file_core::ScalarAssertion::OneOf(allowed, message) => {
            if current
                .and_then(Item::as_str)
                .is_some_and(|value| allowed.iter().map(&render).any(|allowed| allowed == value))
            {
                return;
            }
            let rendered = allowed.iter().map(render).collect::<Vec<_>>();
            findings.push(file_core::Finding::Mismatch {
                key: display_key,
                selector: None,
                current: current.and_then(toml_core::render_item),
                expected: format!("one of {rendered:?}"),
                message: message.clone(),
                severity: file_core::Severity::Error,
                attribution: attribution.to_vec(),
            });
        }
        file_core::ScalarAssertion::Present(message) => {
            if current.is_some() {
                return;
            }
            findings.push(file_core::Finding::Mismatch {
                key: display_key,
                selector: None,
                current: None,
                expected: "present".to_owned(),
                message: message.clone(),
                severity: file_core::Severity::Error,
                attribution: attribution.to_vec(),
            });
        }
        file_core::ScalarAssertion::Absent(message) => {
            let Some(rendered) = current.and_then(toml_core::render_item) else {
                return;
            };
            findings.push(file_core::Finding::Mismatch {
                key: display_key,
                selector: None,
                current: Some(rendered),
                expected: "absent".to_owned(),
                message: message.clone(),
                severity: file_core::Severity::Error,
                attribution: attribution.to_vec(),
            });
            let _ = table.remove(key);
        }
        file_core::ScalarAssertion::AtLeast(..)
        | file_core::ScalarAssertion::AtMost(..)
        | file_core::ScalarAssertion::Range(..) => {}
    }
}

fn apply_list(
    table: &mut Table,
    key: &str,
    resolved: &file_core::ResolvedListRequirements,
    findings: &mut Vec<file_core::Finding>,
) {
    if support::list_is_empty(resolved) {
        return;
    }
    let current_values = toml_core::table_list_values_optional(table, key);
    let mut values = current_values.clone().unwrap_or_default();
    values.sort();
    values.dedup();
    let canonical_changed = current_values
        .as_ref()
        .is_some_and(|current| values != *current);
    let display_key = format!("toolchain.{key}");
    if let Some(mut updated) = toml_core::reconcile_optional_list_field(
        display_key,
        current_values.as_ref().map(|_| values),
        resolved,
        toml_core::ListFieldKeyStyle::FieldItem,
        findings,
    ) {
        updated.sort();
        updated.dedup();
        toml_core::write_table_list(table, key, &updated);
    } else if canonical_changed {
        let mut canonical_values = current_values.unwrap_or_default();
        canonical_values.sort();
        canonical_values.dedup();
        findings.push(file_core::Finding::Mismatch {
            key: format!("toolchain.{key}"),
            selector: None,
            current: table.get(key).and_then(toml_core::render_item),
            expected: format!("{canonical_values:?}"),
            message: "rust-toolchain.toml lists must be canonical.".to_owned(),
            severity: file_core::Severity::Error,
            attribution: support::list_attribution(resolved),
        });
        toml_core::write_table_list(table, key, &canonical_values);
    }
}
