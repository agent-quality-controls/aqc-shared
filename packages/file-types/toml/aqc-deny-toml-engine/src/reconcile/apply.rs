//! Apply resolved deny.toml requirements to a TOML document.

use aqc_file_engine_core::{Finding, Severity};
use toml_edit::{DocumentMut, Item};

use crate::requirement::ResolvedDenyTomlRequirements;

use super::{
    items::apply_items,
    lists::apply_lists,
    scalars::{apply_scalars, touch_core_scalar_helpers},
    support::table_path_mut,
};

pub(crate) fn apply(
    doc: &mut DocumentMut,
    requirement: &ResolvedDenyTomlRequirements,
    findings: &mut Vec<Finding>,
) {
    touch_core_scalar_helpers(findings);
    reject_unsupported_source_key(doc, findings);

    remove_rejected_table_keys(doc, requirement, findings);
    apply_items(doc, requirement, findings);
    apply_scalars(doc, requirement, findings);
    apply_lists(doc, requirement, findings);
    report_missing_table_keys(doc, requirement, findings);
}

fn remove_rejected_table_keys(
    doc: &mut DocumentMut,
    requirement: &ResolvedDenyTomlRequirements,
    findings: &mut Vec<Finding>,
) {
    for (table, keys) in &requirement.table_keys {
        if *table == crate::requirement::DenyTable::Root {
            aqc_toml_engine_core::remove_rejected_table_keys(
                doc.as_table_mut(),
                "",
                keys,
                findings,
            );
            continue;
        }
        let path = table.path();
        if let Some(table_like) = table_path_mut(doc, path) {
            aqc_toml_engine_core::remove_rejected_table_keys(
                table_like,
                table.display_key(),
                keys,
                findings,
            );
        } else {
            let mut absent = toml_edit::Table::new();
            aqc_toml_engine_core::remove_rejected_table_keys(
                &mut absent,
                table.display_key(),
                keys,
                findings,
            );
        }
    }
}

fn report_missing_table_keys(
    doc: &DocumentMut,
    requirement: &ResolvedDenyTomlRequirements,
    findings: &mut Vec<Finding>,
) {
    for (table, keys) in &requirement.table_keys {
        if *table == crate::requirement::DenyTable::Root {
            aqc_toml_engine_core::report_missing_table_keys(doc.as_table(), "", keys, findings);
            continue;
        }
        let path = table.path();
        if let Some(table_like) = super::support::table_path_ref(doc, path) {
            aqc_toml_engine_core::report_missing_table_keys(
                table_like,
                table.display_key(),
                keys,
                findings,
            );
        } else {
            let absent = toml_edit::Table::new();
            aqc_toml_engine_core::report_missing_table_keys(
                &absent,
                table.display_key(),
                keys,
                findings,
            );
        }
    }
}

impl crate::requirement::DenyTable {
    const fn path(self) -> &'static [&'static str] {
        match self {
            Self::Root => &[],
            Self::Graph => &["graph"],
            Self::Output => &["output"],
            Self::Advisories => &["advisories"],
            Self::Licenses => &["licenses"],
            Self::LicensesPrivate => &["licenses", "private"],
            Self::Bans => &["bans"],
            Self::BansWorkspaceDependencies => &["bans", "workspace-dependencies"],
            Self::BansBuild => &["bans", "build"],
            Self::Sources => &["sources"],
            Self::SourcesAllowOrg => &["sources", "allow-org"],
        }
    }

    const fn display_key(self) -> &'static str {
        match self {
            Self::Root => "",
            Self::Graph => "graph",
            Self::Output => "output",
            Self::Advisories => "advisories",
            Self::Licenses => "licenses",
            Self::LicensesPrivate => "licenses.private",
            Self::Bans => "bans",
            Self::BansWorkspaceDependencies => "bans.workspace-dependencies",
            Self::BansBuild => "bans.build",
            Self::Sources => "sources",
            Self::SourcesAllowOrg => "sources.allow-org",
        }
    }
}

fn reject_unsupported_source_key(doc: &mut DocumentMut, findings: &mut Vec<Finding>) {
    let Some(sources) = doc.get_mut("sources").and_then(Item::as_table_mut) else {
        return;
    };
    if sources.remove("unused-allowed-org").is_some() {
        findings.push(Finding::Mismatch {
            key: "sources.unused-allowed-org".to_owned(),
            selector: None,
            current: Some("present".to_owned()),
            expected: "absent".to_owned(),
            message: "unsupported by cargo-deny 0.19.4".to_owned(),
            severity: Severity::Error,
            attribution: Vec::new(),
        });
    }
}
