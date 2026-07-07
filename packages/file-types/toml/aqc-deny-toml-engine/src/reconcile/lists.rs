//! List deny.toml reconciliation.

use aqc_file_engine_core::{Finding, Provenance};
use aqc_toml_engine_core::{
    ListFieldKeyStyle, attribution, push_mismatch, reconcile_list_field, report_list_shape,
};
use toml_edit::{DocumentMut, Item};

use crate::requirement::ResolvedDenyTomlRequirements;

use super::support::{ensure_table_path, render_item, string_array_item, table_item};

pub(super) fn apply_lists(
    doc: &mut DocumentMut,
    requirement: &ResolvedDenyTomlRequirements,
    findings: &mut Vec<Finding>,
) {
    apply_list(
        doc,
        &["graph"],
        "exclude",
        "graph.exclude",
        &requirement.graph_exclude,
        findings,
    );
    apply_list(
        doc,
        &["graph"],
        "features",
        "graph.features",
        &requirement.graph_features,
        findings,
    );
    apply_list(
        doc,
        &["advisories"],
        "db-urls",
        "advisories.db-urls",
        &requirement.advisories_db_urls,
        findings,
    );
    apply_list(
        doc,
        &["licenses"],
        "allow",
        "licenses.allow",
        &requirement.licenses_allow,
        findings,
    );
    apply_list(
        doc,
        &["licenses", "private"],
        "registries",
        "licenses.private.registries",
        &requirement.licenses_private_registries,
        findings,
    );
    apply_list(
        doc,
        &["licenses", "private"],
        "ignore-sources",
        "licenses.private.ignore-sources",
        &requirement.licenses_private_ignore_sources,
        findings,
    );
    apply_list(
        doc,
        &["bans", "build"],
        "script-extensions",
        "bans.build.script-extensions",
        &requirement.bans_build_script_extensions,
        findings,
    );
    apply_list(
        doc,
        &["sources"],
        "allow-git",
        "sources.allow-git",
        &requirement.sources_allow_git,
        findings,
    );
    apply_list(
        doc,
        &["sources"],
        "private",
        "sources.private",
        &requirement.sources_private,
        findings,
    );
    apply_list(
        doc,
        &["sources"],
        "allow-registry",
        "sources.allow-registry",
        &requirement.sources_allow_registry,
        findings,
    );
    apply_list(
        doc,
        &["sources", "allow-org"],
        "github",
        "sources.allow-org.github",
        &requirement.sources_allow_org_github,
        findings,
    );
    apply_list(
        doc,
        &["sources", "allow-org"],
        "gitlab",
        "sources.allow-org.gitlab",
        &requirement.sources_allow_org_gitlab,
        findings,
    );
    apply_list(
        doc,
        &["sources", "allow-org"],
        "bitbucket",
        "sources.allow-org.bitbucket",
        &requirement.sources_allow_org_bitbucket,
        findings,
    );
}

fn apply_list(
    doc: &mut DocumentMut,
    table_path: &[&str],
    field_key: &str,
    display_key: &str,
    requirement: &aqc_file_engine_core::ResolvedListRequirements,
    findings: &mut Vec<Finding>,
) {
    if requirement.contains.is_empty()
        && requirement.excludes.is_empty()
        && requirement.exact.is_none()
    {
        return;
    }

    report_nested_list_shape(
        doc,
        table_path,
        field_key,
        display_key,
        requirement,
        findings,
    );
    let current = table_item(doc, table_path, field_key)
        .and_then(Item::as_array)
        .map(|array| {
            array
                .iter()
                .filter_map(|value| value.as_str().map(ToOwned::to_owned))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let Some(updated) = reconcile_list_field(
        display_key.to_owned(),
        current,
        requirement,
        ListFieldKeyStyle::FieldItem,
        findings,
    ) else {
        return;
    };
    let table = ensure_table_path(doc, table_path);
    table[field_key] = string_array_item(&updated);
}

fn report_nested_list_shape(
    doc: &DocumentMut,
    table_path: &[&str],
    field_key: &str,
    display_key: &str,
    requirement: &aqc_file_engine_core::ResolvedListRequirements,
    findings: &mut Vec<Finding>,
) {
    if table_path.is_empty() {
        let _ = report_list_shape(doc, display_key, requirement, findings);
        return;
    }
    let Some(item) = table_item(doc, table_path, field_key) else {
        return;
    };
    let attribution = list_attribution(requirement);
    let message = list_message(requirement);
    let Some(array) = item.as_array() else {
        push_mismatch(
            findings,
            display_key.to_owned(),
            Some(render_item(item)),
            "array of strings".to_owned(),
            message,
            &attribution,
        );
        return;
    };
    for (index, value) in array.iter().enumerate() {
        if value.as_str().is_some() {
            continue;
        }
        push_mismatch(
            findings,
            format!("{display_key}[{index}]"),
            Some(value.to_string()),
            "string".to_owned(),
            message.clone(),
            &attribution,
        );
    }
}

fn list_message(requirement: &aqc_file_engine_core::ResolvedListRequirements) -> String {
    requirement
        .contains
        .values()
        .flat_map(|resolved| resolved.collected.iter().map(|(_, msg)| msg.as_str()))
        .chain(
            requirement
                .excludes
                .values()
                .flat_map(|resolved| resolved.collected.iter().map(|(_, msg)| msg.as_str())),
        )
        .chain(
            requirement
                .exact
                .iter()
                .flat_map(|resolved| resolved.collected.iter().map(|(_, (_, msg))| msg.as_str())),
        )
        .next()
        .unwrap_or_default()
        .to_owned()
}

fn list_attribution(
    requirement: &aqc_file_engine_core::ResolvedListRequirements,
) -> Vec<Provenance> {
    requirement
        .contains
        .values()
        .flat_map(attribution)
        .chain(requirement.excludes.values().flat_map(attribution))
        .chain(requirement.exact.iter().flat_map(attribution))
        .collect()
}
