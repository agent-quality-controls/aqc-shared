//! Reconcile table-level existence per `ManifestSection`.
//!
//! `Present`: the table must exist. For every section except `Package` the
//! engine writes an empty table when missing; `Package` is check-only (it
//! needs a `name` the engine cannot invent). `Absent`: remove the whole table
//! when present (vacuous when already missing).

#![allow(
    clippy::type_complexity,
    reason = "Section reconciliation consumes resolved map requirement shapes."
)]

use std::collections::BTreeMap;

use aqc_file_engine_core::{Finding, Provenance, ResolvedRequirement, Severity};
use aqc_toml_engine_core::{ensure_nested, ensure_table};
use toml_edit::{DocumentMut, Item, Table};

use crate::requirement::{ManifestSection, SectionPresenceAssertion};

/// Apply every section-presence requirement.
pub(crate) fn apply(
    doc: &mut DocumentMut,
    merged_by_section: &BTreeMap<
        ManifestSection,
        ResolvedRequirement<SectionPresenceAssertion, SectionPresenceAssertion>,
    >,
    findings: &mut Vec<Finding>,
) {
    for (section, merged) in merged_by_section {
        let attribution = merged.attribution();
        apply_one(doc, *section, &merged.merged, &attribution, findings);
    }
}

/// Apply a single `SectionPresenceAssertion` for one section.
fn apply_one(
    doc: &mut DocumentMut,
    section: ManifestSection,
    assertion: &SectionPresenceAssertion,
    attribution: &[Provenance],
    findings: &mut Vec<Finding>,
) {
    match assertion {
        SectionPresenceAssertion::Present(msg) => {
            apply_present(doc, section, msg, attribution, findings);
        }
        SectionPresenceAssertion::Absent(msg) => {
            apply_absent(doc, section, msg, attribution, findings);
        }
    }
}

/// The section's table must exist. `Package` is check-only.
fn apply_present(
    doc: &mut DocumentMut,
    section: ManifestSection,
    msg: &str,
    attribution: &[Provenance],
    findings: &mut Vec<Finding>,
) {
    if section_exists(doc, section) {
        return;
    }
    findings.push(Finding::Mismatch {
        key: section.table_path().to_owned(),
        selector: None,
        current: None,
        expected: "table present".to_owned(),
        message: msg.to_owned(),
        severity: Severity::Error,
        attribution: attribution.to_vec(),
    });
    // `[package]` needs a `name` the engine cannot invent: check-only.
    if matches!(section, ManifestSection::Package) {
        return;
    }
    ensure_section(doc, section);
}

/// The section's table must not exist (vacuous when already absent).
fn apply_absent(
    doc: &mut DocumentMut,
    section: ManifestSection,
    msg: &str,
    attribution: &[Provenance],
    findings: &mut Vec<Finding>,
) {
    if !section_exists(doc, section) {
        return;
    }
    findings.push(Finding::Mismatch {
        key: section.table_path().to_owned(),
        selector: None,
        current: Some("table present".to_owned()),
        expected: "table absent".to_owned(),
        message: msg.to_owned(),
        severity: Severity::Error,
        attribution: attribution.to_vec(),
    });
    remove_section(doc, section);
}

/// Return whether the manifest currently contains the section table.
fn section_exists(doc: &DocumentMut, section: ManifestSection) -> bool {
    if matches!(section, ManifestSection::WorkspaceLints) {
        return doc
            .get("workspace")
            .and_then(Item::as_table_like)
            .and_then(|workspace| workspace.get("lints"))
            .is_some();
    }
    doc.get(section.key()).is_some()
}

/// Insert the section table when it can be initialized without inventing
/// Cargo-owned semantic fields.
fn ensure_section(doc: &mut DocumentMut, section: ManifestSection) {
    if matches!(section, ManifestSection::WorkspaceLints) {
        let workspace = ensure_table(doc, "workspace");
        let _ = ensure_nested(workspace, "lints");
        return;
    }
    let _ = doc
        .entry(section.key())
        .or_insert(Item::Table(Table::new()));
}

/// Remove the section table from the manifest when it exists.
fn remove_section(doc: &mut DocumentMut, section: ManifestSection) {
    if matches!(section, ManifestSection::WorkspaceLints) {
        if let Some(workspace) = doc.get_mut("workspace").and_then(Item::as_table_mut) {
            let _ = workspace.remove("lints");
        }
        return;
    }
    let _ = doc.remove(section.key());
}
