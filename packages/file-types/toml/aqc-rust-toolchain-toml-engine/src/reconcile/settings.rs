//! Reconcile `[toolchain]` settings.

use std::collections::BTreeSet;
use std::path::Path;

use aqc_file_engine_core as file_core;
use aqc_toml_engine_core as toml_core;
use toml_edit::{DocumentMut, Item, Table};

use crate::requirement::{
    ResolvedRustToolchainTomlRequirements, RustToolchainListSetting, RustToolchainScalarSetting,
};

type ResolvedRustToolchainScalarSetting = file_core::ResolvedRequirement<
    file_core::ScalarAssertion<file_core::ConfigScalar>,
    file_core::ScalarAssertion<file_core::ConfigScalar>,
>;

pub(crate) fn apply(
    doc: &mut DocumentMut,
    requirement: &ResolvedRustToolchainTomlRequirements,
    findings: &mut Vec<file_core::Finding>,
) {
    report_invalid_requirement_combinations(requirement, findings);
    let table = ensure_toolchain_table(doc, requirement, findings);
    report_existing_file_conflicts(table, requirement, findings);

    if has_path_value_requirement(requirement) {
        apply_path_setting(table, requirement, findings);
        apply_closed(table, requirement, findings);
        report_empty_table(table, findings);
        return;
    }

    apply_scalar_settings(table, requirement, findings);
    apply_list_settings(table, requirement, findings);
    apply_closed(table, requirement, findings);
    report_empty_table(table, findings);
}

fn ensure_toolchain_table<'a>(
    doc: &'a mut DocumentMut,
    requirement: &ResolvedRustToolchainTomlRequirements,
    findings: &mut Vec<file_core::Finding>,
) -> &'a mut Table {
    if doc.get("toolchain").and_then(Item::as_table).is_none() {
        if let Some(item) = doc.get("toolchain") {
            toml_core::push_mismatch(
                findings,
                "toolchain".to_owned(),
                toml_core::render_item(item),
                "table".to_owned(),
                "rust-toolchain.toml must contain a [toolchain] table.".to_owned(),
                &requirement_attribution(requirement),
            );
        } else if has_requirements(requirement) {
            toml_core::push_mismatch(
                findings,
                "toolchain".to_owned(),
                None,
                "table".to_owned(),
                "rust-toolchain.toml must contain a [toolchain] table.".to_owned(),
                &requirement_attribution(requirement),
            );
        }
    }
    toml_core::ensure_table(doc, "toolchain")
}

fn apply_scalar_settings(
    table: &mut Table,
    requirement: &ResolvedRustToolchainTomlRequirements,
    findings: &mut Vec<file_core::Finding>,
) {
    for (setting, resolved) in &requirement.scalar_settings {
        if *setting == RustToolchainScalarSetting::Profile {
            report_invalid_profile(table, resolved, findings);
        }
        if *setting == RustToolchainScalarSetting::Path {
            if report_invalid_path_requirement(resolved, findings) {
                continue;
            }
        }
        apply_scalar_setting(table, *setting, resolved, findings);
    }
}

fn apply_path_setting(
    table: &mut Table,
    requirement: &ResolvedRustToolchainTomlRequirements,
    findings: &mut Vec<file_core::Finding>,
) {
    let Some(resolved) = requirement
        .scalar_settings
        .get(&RustToolchainScalarSetting::Path)
    else {
        return;
    };
    if report_invalid_path_requirement(resolved, findings) {
        return;
    }
    apply_scalar_setting(table, RustToolchainScalarSetting::Path, resolved, findings);
}

fn apply_scalar_setting(
    table: &mut Table,
    setting: RustToolchainScalarSetting,
    resolved: &ResolvedRustToolchainScalarSetting,
    findings: &mut Vec<file_core::Finding>,
) {
    let key = setting.file_key();
    let display_key = format!("toolchain.{key}");
    let attribution = scalar_attribution_for(table, key, resolved);
    match toml_core::scalar_field_edit(
        display_key,
        table.get(key),
        &resolved.merged,
        &attribution,
        findings,
    ) {
        Some(toml_core::ScalarFieldEdit::Write(item)) => {
            table[key] = item;
        }
        Some(toml_core::ScalarFieldEdit::Remove) => {
            let _ = table.remove(key);
        }
        None => {}
    }
}

fn apply_list_settings(
    table: &mut Table,
    requirement: &ResolvedRustToolchainTomlRequirements,
    findings: &mut Vec<file_core::Finding>,
) {
    for (setting, resolved) in &requirement.list_settings {
        let key = setting.file_key();
        if report_table_list_shape(table, key, resolved, findings) {
            let mut values = toml_core::table_list_values(table, key);
            values.sort();
            values.dedup();
            toml_core::write_table_list(table, key, &values);
        }
        let mut values = toml_core::table_list_values(table, key);
        values.sort();
        values.dedup();
        if let Some(mut updated) = toml_core::reconcile_list_field(
            format!("toolchain.{key}"),
            values,
            resolved,
            toml_core::ListFieldKeyStyle::FieldItem,
            findings,
        ) {
            updated.sort();
            updated.dedup();
            toml_core::write_table_list(table, key, &updated);
        }
    }
}

fn report_table_list_shape(
    table: &Table,
    key: &str,
    requirements: &file_core::ResolvedListRequirements,
    findings: &mut Vec<file_core::Finding>,
) -> bool {
    let Some(item) = table.get(key) else {
        return false;
    };
    let attr = list_attribution(requirements);
    if item.as_array().is_none() {
        toml_core::push_mismatch(
            findings,
            format!("toolchain.{key}"),
            toml_core::render_item(item),
            "array of strings".to_owned(),
            toml_core::list_message(requirements),
            &attr,
        );
        return true;
    }
    let mut malformed = false;
    for (index, value) in item.as_array().into_iter().flatten().enumerate() {
        if value.as_str().is_some() {
            continue;
        }
        malformed = true;
        toml_core::push_mismatch(
            findings,
            format!("toolchain.{key}[{index}]"),
            Some(value.to_string()),
            "string".to_owned(),
            toml_core::list_message(requirements),
            &attr,
        );
    }
    malformed
}

fn report_invalid_requirement_combinations(
    requirement: &ResolvedRustToolchainTomlRequirements,
    findings: &mut Vec<file_core::Finding>,
) {
    let path = requirement
        .scalar_settings
        .get(&RustToolchainScalarSetting::Path);
    if !has_path_value_requirement(requirement) {
        return;
    }
    if let Some(channel) = requirement
        .scalar_settings
        .get(&RustToolchainScalarSetting::Channel)
    {
        findings.push(invalid_requirements(
            "toolchain.path",
            "`path` and `channel` cannot both be required.",
            path.into_iter()
                .chain(Some(channel))
                .flat_map(|resolved| resolved.collected.iter())
                .map(|(prov, assertion)| (prov.policy.clone(), assertion.message().to_owned()))
                .collect(),
        ));
    }
    for setting in [
        RustToolchainScalarSetting::Profile,
        RustToolchainScalarSetting::Channel,
    ] {
        if setting == RustToolchainScalarSetting::Channel {
            continue;
        }
        if let Some(resolved) = requirement.scalar_settings.get(&setting) {
            findings.push(invalid_requirements(
                format!("toolchain.{}", setting.file_key()),
                "`path` disables channel-based toolchain settings.",
                path.into_iter()
                    .chain(Some(resolved))
                    .flat_map(|resolved| resolved.collected.iter())
                    .map(|(prov, assertion)| (prov.policy.clone(), assertion.message().to_owned()))
                    .collect(),
            ));
        }
    }
    for (setting, resolved) in &requirement.list_settings {
        findings.push(invalid_requirements(
            format!("toolchain.{}", setting.file_key()),
            "`path` disables channel-based toolchain settings.",
            path.into_iter()
                .flat_map(|resolved| resolved.collected.iter())
                .map(|(prov, assertion)| (prov.policy.clone(), assertion.message().to_owned()))
                .chain(list_contributors(resolved))
                .collect(),
        ));
    }
}

fn report_existing_file_conflicts(
    table: &mut Table,
    requirement: &ResolvedRustToolchainTomlRequirements,
    findings: &mut Vec<file_core::Finding>,
) {
    if table.contains_key("channel") && table.contains_key("path") {
        toml_core::push_mismatch(
            findings,
            "toolchain.path".to_owned(),
            table.get("path").and_then(toml_core::render_item),
            "absent when channel is set".to_owned(),
            "`path` and `channel` cannot both be present.".to_owned(),
            &requirement_attribution(requirement),
        );
        let _ = table.remove("path");
    }
    if table.contains_key("path") && has_channel_only_requirements(requirement) {
        toml_core::push_mismatch(
            findings,
            "toolchain.path".to_owned(),
            table.get("path").and_then(toml_core::render_item),
            "absent when channel-based settings are required".to_owned(),
            "`path` disables channel-based toolchain settings.".to_owned(),
            &requirement_attribution(requirement),
        );
        let _ = table.remove("path");
    }
    report_relative_path_file(table, requirement, findings);
}

fn report_invalid_profile(
    table: &Table,
    resolved: &ResolvedRustToolchainScalarSetting,
    findings: &mut Vec<file_core::Finding>,
) {
    if let Some(value) = table.get("profile").and_then(Item::as_str) {
        if matches!(value, "minimal" | "default" | "complete") {
            return;
        }
        toml_core::push_mismatch(
            findings,
            "toolchain.profile".to_owned(),
            Some(value.to_owned()),
            "one of [\"minimal\", \"default\", \"complete\"]".to_owned(),
            "rust-toolchain.toml profile must be accepted by rustup.".to_owned(),
            &toml_core::attribution(resolved),
        );
    }
}

fn report_invalid_path_requirement(
    resolved: &ResolvedRustToolchainScalarSetting,
    findings: &mut Vec<file_core::Finding>,
) -> bool {
    if let file_core::ScalarAssertion::Equals(file_core::ConfigScalar::Str(value), message) =
        &resolved.merged
    {
        if Path::new(value).is_absolute() {
            return false;
        }
        findings.push(file_core::Finding::InvalidRequirements {
            key: "toolchain.path".to_owned(),
            message: "toolchain path requirements must be absolute.".to_owned(),
            contributors: resolved
                .collected
                .iter()
                .map(|(prov, _)| (prov.policy.clone(), message.clone()))
                .collect(),
        });
        return true;
    }
    false
}

fn report_relative_path_file(
    table: &Table,
    requirement: &ResolvedRustToolchainTomlRequirements,
    findings: &mut Vec<file_core::Finding>,
) {
    let Some(value) = table.get("path").and_then(Item::as_str) else {
        return;
    };
    if Path::new(value).is_absolute() {
        return;
    }
    toml_core::push_mismatch(
        findings,
        "toolchain.path".to_owned(),
        Some(value.to_owned()),
        "absolute path".to_owned(),
        "rust-toolchain.toml path must be absolute.".to_owned(),
        &requirement_attribution(requirement),
    );
}

fn apply_closed(
    table: &mut Table,
    requirement: &ResolvedRustToolchainTomlRequirements,
    findings: &mut Vec<file_core::Finding>,
) {
    if requirement.closed_settings.is_empty() {
        return;
    }
    let allowed = requirement
        .scalar_settings
        .keys()
        .map(|key| key.file_key())
        .chain(requirement.list_settings.keys().map(|key| key.file_key()))
        .collect::<BTreeSet<_>>();
    let extras = table
        .iter()
        .map(|(key, _)| key.to_owned())
        .filter(|key| !allowed.contains(key.as_str()))
        .collect::<Vec<_>>();
    for extra in extras {
        toml_core::push_mismatch(
            findings,
            format!("toolchain.{extra}"),
            table.get(&extra).and_then(toml_core::render_item),
            "absent because rust-toolchain.toml settings are closed".to_owned(),
            requirement
                .closed_settings
                .first()
                .map(|(_, msg)| msg.clone())
                .unwrap_or_default(),
            &requirement
                .closed_settings
                .iter()
                .map(|(prov, _)| prov.clone())
                .collect::<Vec<_>>(),
        );
        let _ = table.remove(&extra);
    }
}

fn report_empty_table(table: &Table, findings: &mut Vec<file_core::Finding>) {
    if !table.is_empty() {
        return;
    }
    findings.push(file_core::Finding::Mismatch {
        key: "toolchain".to_owned(),
        current: Some("{}".to_owned()),
        expected: "at least one supported property".to_owned(),
        message: "rust-toolchain.toml [toolchain] table cannot be empty.".to_owned(),
        severity: file_core::Severity::Error,
        attribution: Vec::new(),
    });
}

fn scalar_attribution_for(
    table: &Table,
    key: &str,
    resolved: &ResolvedRustToolchainScalarSetting,
) -> Vec<file_core::Provenance> {
    let current = table.get(key);
    let filtered = resolved
        .collected
        .iter()
        .filter(|(_, assertion)| toml_core::scalar_assertion_fails(current, assertion))
        .map(|(prov, _)| prov.clone())
        .collect::<Vec<_>>();
    if filtered.is_empty() {
        toml_core::attribution(resolved)
    } else {
        filtered
    }
}

fn list_attribution(
    requirements: &file_core::ResolvedListRequirements,
) -> Vec<file_core::Provenance> {
    requirements
        .contains
        .values()
        .flat_map(toml_core::attribution)
        .chain(
            requirements
                .excludes
                .values()
                .flat_map(toml_core::attribution),
        )
        .chain(requirements.exact.iter().flat_map(toml_core::attribution))
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

fn requirement_attribution(
    requirement: &ResolvedRustToolchainTomlRequirements,
) -> Vec<file_core::Provenance> {
    requirement
        .scalar_settings
        .values()
        .flat_map(toml_core::attribution)
        .chain(
            requirement
                .list_settings
                .values()
                .flat_map(list_attribution),
        )
        .chain(
            requirement
                .closed_settings
                .iter()
                .map(|(prov, _)| prov.clone()),
        )
        .collect()
}

fn has_requirements(requirement: &ResolvedRustToolchainTomlRequirements) -> bool {
    !requirement.scalar_settings.is_empty()
        || !requirement.list_settings.is_empty()
        || !requirement.closed_settings.is_empty()
}

fn has_path_value_requirement(requirement: &ResolvedRustToolchainTomlRequirements) -> bool {
    requirement
        .scalar_settings
        .get(&RustToolchainScalarSetting::Path)
        .is_some_and(|resolved| !matches!(resolved.merged, file_core::ScalarAssertion::Absent(_)))
}

fn has_channel_only_requirements(requirement: &ResolvedRustToolchainTomlRequirements) -> bool {
    requirement
        .scalar_settings
        .contains_key(&RustToolchainScalarSetting::Profile)
        || requirement
            .scalar_settings
            .contains_key(&RustToolchainScalarSetting::Channel)
        || requirement
            .list_settings
            .contains_key(&RustToolchainListSetting::Components)
        || requirement
            .list_settings
            .contains_key(&RustToolchainListSetting::Targets)
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
