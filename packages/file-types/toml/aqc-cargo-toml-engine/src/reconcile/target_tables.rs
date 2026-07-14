//! Reconcile target tables: `[lib].<field>` and the named `[[bin]]` /
//! `[[example]]` / `[[test]]` / `[[bench]]` entries (identified by `name`).
//!
//! Lazy: check-only assertions (`OneOf`, `Present` on a field) and vacuous
//! removals never create tables or entries. A `Fields` assertion with any
//! check-only field does not create a missing entry (it cannot be satisfied
//! by writing); all-writable `Fields` create the entry with its name.

#![cfg_attr(
    not(test),
    expect(
        clippy::missing_docs_in_private_items,
        reason = "Private target-table helpers are internal reconciliation steps."
    )
)]
#![allow(
    clippy::type_complexity,
    reason = "Target table reconciliation keeps nested Cargo target fields in one traversal."
)]

use std::collections::BTreeMap;

use aqc_file_engine_core::{
    ConfigScalar, Finding, OnEmpty, OnEmptyClass, Provenance, ResolvedRequirement, ScalarAssertion,
    Severity,
};
use aqc_toml_engine_core::{ScalarFieldEdit, scalar_field_edit};
use toml_edit::{DocumentMut, Item, Table, value};

use crate::requirement::{
    ResolvedTargetFieldAssertion, ResolvedTargetTableAssertion, TargetFieldAssertion,
};

/// Where a target field lives: the singleton `[lib]` table or a named
/// array-of-tables entry.
enum FieldLoc<'a> {
    /// `[lib]`.
    Lib,
    /// `[[<kind>]]` entry with this `name`.
    Entry {
        /// The array key (`bin`, `example`, `test`, `bench`).
        kind: &'a str,
        /// The entry's `name` value.
        name: &'a str,
    },
}

impl FieldLoc<'_> {
    /// The finding-path prefix for this location.
    fn prefix(&self) -> String {
        match self {
            Self::Lib => "[lib]".to_owned(),
            Self::Entry { kind, name } => format!("[[{kind}]].{name}"),
        }
    }
}

/// Apply every `[lib].<field>` requirement.
pub(crate) fn apply_lib(
    doc: &mut DocumentMut,
    merged_by_field: &BTreeMap<
        String,
        ResolvedRequirement<ResolvedTargetFieldAssertion, crate::requirement::TargetFieldAssertion>,
    >,
    findings: &mut Vec<Finding>,
) {
    for (field, merged) in merged_by_field {
        let attribution = field_attribution_for(doc, &FieldLoc::Lib, field, merged);
        apply_field(
            doc,
            &FieldLoc::Lib,
            field,
            &merged.merged,
            &attribution,
            findings,
        );
    }
}

/// Apply every named `[[<kind>]]` target requirement.
pub(crate) fn apply_named(
    doc: &mut DocumentMut,
    kind: &str,
    merged_by_name: &BTreeMap<
        String,
        ResolvedRequirement<ResolvedTargetTableAssertion, crate::requirement::TargetTableAssertion>,
    >,
    findings: &mut Vec<Finding>,
) {
    for (name, merged) in merged_by_name {
        let attribution = merged.attribution();
        apply_named_one(doc, kind, name, &merged.merged, &attribution, findings);
    }
}

/// Apply one resolved target table assertion against the named entry.
fn apply_named_one(
    doc: &mut DocumentMut,
    kind: &str,
    name: &str,
    assertion: &ResolvedTargetTableAssertion,
    attribution: &[Provenance],
    findings: &mut Vec<Finding>,
) {
    let exists = entry_index(doc, kind, name).is_some();
    match assertion {
        ResolvedTargetTableAssertion::Present(msg) => {
            if exists {
                return;
            }
            findings.push(Finding::Mismatch {
                key: format!("[[{kind}]].{name}"),
                selector: None,
                current: None,
                expected: "present".to_owned(),
                message: msg.clone(),
                severity: Severity::Error,
                attribution: attribution.to_vec(),
            });
            let _ = ensure_entry(doc, kind, name);
        }
        ResolvedTargetTableAssertion::Absent(msg) => {
            let Some(index) = entry_index(doc, kind, name) else {
                return;
            };
            findings.push(Finding::Mismatch {
                key: format!("[[{kind}]].{name}"),
                selector: None,
                current: Some("present".to_owned()),
                expected: "absent".to_owned(),
                message: msg.clone(),
                severity: Severity::Error,
                attribution: attribution.to_vec(),
            });
            if let Some(aot) = doc.get_mut(kind).and_then(Item::as_array_of_tables_mut) {
                let _ = aot.remove(index);
            }
        }
        ResolvedTargetTableAssertion::Fields(map) => {
            if !exists && assertion.on_empty() == OnEmpty::ChecksOnly {
                // Cannot be satisfied by writing: report the missing entry only.
                findings.push(Finding::Mismatch {
                    key: format!("[[{kind}]].{name}"),
                    selector: None,
                    current: None,
                    expected: "present (check-only fields)".to_owned(),
                    message: String::new(),
                    severity: Severity::Error,
                    attribution: attribution.to_vec(),
                });
                return;
            }
            if !exists {
                findings.push(Finding::Mismatch {
                    key: format!("[[{kind}]].{name}"),
                    selector: None,
                    current: None,
                    expected: "present".to_owned(),
                    message: String::new(),
                    severity: Severity::Error,
                    attribution: attribution.to_vec(),
                });
                let _ = ensure_entry(doc, kind, name);
            }
            let loc = FieldLoc::Entry { kind, name };
            for (field, field_resolved) in map {
                let field_attribution = field_attribution_for(doc, &loc, field, field_resolved);
                apply_field(
                    doc,
                    &loc,
                    field,
                    &field_resolved.merged,
                    &field_attribution,
                    findings,
                );
            }
        }
    }
}

fn field_attribution_for(
    doc: &DocumentMut,
    loc: &FieldLoc<'_>,
    field: &str,
    resolved: &ResolvedRequirement<ResolvedTargetFieldAssertion, TargetFieldAssertion>,
) -> Vec<Provenance> {
    let current = read_field(doc, loc, field);
    let filtered = resolved
        .collected
        .iter()
        .filter(|(_, assertion)| field_assertion_fails(current, assertion))
        .map(|(prov, _)| prov.clone())
        .collect::<Vec<_>>();
    if filtered.is_empty() {
        resolved.attribution()
    } else {
        filtered
    }
}

fn field_assertion_fails(current: Option<&Item>, assertion: &TargetFieldAssertion) -> bool {
    match assertion {
        TargetFieldAssertion::Scalar(assertion) => {
            aqc_toml_engine_core::scalar_assertion_fails(current, assertion)
        }
        TargetFieldAssertion::List(_) => false,
    }
}

/// Apply one resolved target field assertion at `loc`.
fn apply_field(
    doc: &mut DocumentMut,
    loc: &FieldLoc<'_>,
    field: &str,
    assertion: &ResolvedTargetFieldAssertion,
    attribution: &[Provenance],
    findings: &mut Vec<Finding>,
) {
    let path = format!("{}.{field}", loc.prefix());
    match assertion {
        ResolvedTargetFieldAssertion::Scalar(assertion) => {
            apply_scalar_field(doc, loc, field, path, assertion, attribution, findings);
        }
        ResolvedTargetFieldAssertion::List(list) => {
            let current = read_field_array(doc, loc, field);
            if let Some(updated) =
                aqc_toml_engine_core::reconcile_table_list_field(path, current, list, findings)
            {
                aqc_toml_engine_core::write_table_list(ensure_loc(doc, loc), field, &updated);
            }
        }
    }
}

fn apply_scalar_field(
    doc: &mut DocumentMut,
    loc: &FieldLoc<'_>,
    field: &str,
    path: String,
    assertion: &ScalarAssertion<ConfigScalar>,
    attribution: &[Provenance],
    findings: &mut Vec<Finding>,
) {
    match scalar_field_edit(
        path,
        read_field(doc, loc, field),
        assertion,
        attribution,
        findings,
    ) {
        Some(ScalarFieldEdit::Write(item)) => {
            ensure_loc(doc, loc)[field] = item;
        }
        Some(ScalarFieldEdit::Remove) => {
            let _ = ensure_loc(doc, loc).remove(field);
        }
        None => {}
    }
}

/// Read the current item for `field` at `loc`, if any.
fn read_field<'a>(doc: &'a DocumentMut, loc: &FieldLoc<'_>, field: &str) -> Option<&'a Item> {
    match loc {
        FieldLoc::Lib => aqc_toml_engine_core::table_ref(doc, "lib")?.get(field),
        FieldLoc::Entry { kind, name } => entry_ref(doc, kind, name)?.get(field),
    }
}

/// Read the current string array for `field` at `loc` (empty when absent).
fn read_field_array(doc: &DocumentMut, loc: &FieldLoc<'_>, field: &str) -> Vec<String> {
    match loc {
        FieldLoc::Lib => aqc_toml_engine_core::table_ref(doc, "lib").map_or_else(Vec::new, |t| {
            aqc_toml_engine_core::table_list_values(t, field)
        }),
        FieldLoc::Entry { kind, name } => entry_ref(doc, kind, name).map_or_else(Vec::new, |t| {
            aqc_toml_engine_core::table_list_values(t, field)
        }),
    }
}

/// The mutable table at `loc`, created lazily (only called on a write).
fn ensure_loc<'a>(doc: &'a mut DocumentMut, loc: &FieldLoc<'_>) -> &'a mut Table {
    match loc {
        FieldLoc::Lib => aqc_toml_engine_core::ensure_table(doc, "lib"),
        FieldLoc::Entry { kind, name } => ensure_entry(doc, kind, name),
    }
}

/// The named entry's table, if it exists.
fn entry_ref<'a>(doc: &'a DocumentMut, kind: &str, name: &str) -> Option<&'a Table> {
    doc.get(kind)?
        .as_array_of_tables()?
        .iter()
        .find(|t| t.get("name").and_then(Item::as_str) == Some(name))
}

/// The index of the named entry, if it exists.
fn entry_index(doc: &DocumentMut, kind: &str, name: &str) -> Option<usize> {
    doc.get(kind)?
        .as_array_of_tables()?
        .iter()
        .position(|t| t.get("name").and_then(Item::as_str) == Some(name))
}

/// The named entry's table, created (with its `name` key) when missing. Call
/// only when a write is about to happen.
#[expect(
    clippy::expect_used,
    reason = "the entry is pushed just above when missing, so the lookup cannot fail"
)]
fn ensure_entry<'a>(doc: &'a mut DocumentMut, kind: &str, name: &str) -> &'a mut Table {
    let index = entry_index(doc, kind, name);
    let aot = aqc_toml_engine_core::ensure_array_of_tables(doc, kind);
    let index = index.unwrap_or_else(|| {
        let mut t = Table::new();
        let _ = t.insert("name", value(name.to_owned()));
        aot.push(t);
        aot.len().saturating_sub(1)
    });
    aot.get_mut(index)
        .expect("index points at an existing or just-pushed entry")
}
