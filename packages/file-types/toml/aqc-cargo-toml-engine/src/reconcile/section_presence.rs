//! Reconcile table-level existence per `ManifestSection`.
//!
//! `Present`: the table must exist. For every section except `Package` the
//! engine writes an empty table when missing; `Package` is check-only (it
//! needs a `name` the engine cannot invent). `Absent`: remove the whole table
//! when present (vacuous when already missing).

use std::collections::BTreeMap;

use aqc_file_engine_core::{Finding, MergedAssertion, Provenance};
use toml_edit::{DocumentMut, Item, Table};

use crate::reconcile::util::{all_provenances, push_mismatch};
use crate::requirement::{ManifestSection, SectionPresenceAssertion};

/// Apply every section-presence contribution.
#[expect(
    clippy::type_complexity,
    reason = "BTreeMap<ManifestSection, MergedAssertion<...>> is the natural section input shape"
)]
pub(crate) fn apply(
    doc: &mut DocumentMut,
    merged_by_section: &BTreeMap<ManifestSection, MergedAssertion<SectionPresenceAssertion>>,
    findings: &mut Vec<Finding>,
) {
    for (section, merged) in merged_by_section {
        let attribution = all_provenances(merged);
        for (_, assertion) in &merged.contributions {
            apply_one(doc, *section, assertion, &attribution, findings);
        }
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
    let key = section.key();
    if doc.get(key).is_some() {
        return;
    }
    push_mismatch(
        findings,
        section.table_path().to_owned(),
        None,
        "table present".to_owned(),
        msg.to_owned(),
        attribution,
    );
    // `[package]` needs a `name` the engine cannot invent: check-only.
    if matches!(section, ManifestSection::Package) {
        return;
    }
    let _ = doc.entry(key).or_insert(Item::Table(Table::new()));
}

/// The section's table must not exist (vacuous when already absent).
fn apply_absent(
    doc: &mut DocumentMut,
    section: ManifestSection,
    msg: &str,
    attribution: &[Provenance],
    findings: &mut Vec<Finding>,
) {
    let key = section.key();
    if doc.get(key).is_none() {
        return;
    }
    push_mismatch(
        findings,
        section.table_path().to_owned(),
        Some("table present".to_owned()),
        "table absent".to_owned(),
        msg.to_owned(),
        attribution,
    );
    let _ = doc.remove(key);
}
