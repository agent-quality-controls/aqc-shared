//! Reconcile target tables: `[lib].<field>` and the named `[[bin]]` /
//! `[[example]]` / `[[test]]` / `[[bench]]` entries (identified by `name`).
//!
//! Lazy: check-only assertions (`OneOf`, `Present` on a field) and vacuous
//! removals never create tables or entries. A `Fields` assertion with any
//! check-only field does not create a missing entry (it cannot be satisfied
//! by writing); all-writable `Fields` create the entry with its name.

use std::collections::{BTreeMap, BTreeSet};

use aqc_file_engine_core::{Finding, OnEmpty, OnEmptyClass, Provenance, ResolvedRequirement};
use toml_edit::{DocumentMut, Item, Table, value};

use crate::reconcile::util;
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
        let attribution = util::attribution(merged);
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
            util::push_mismatch(
                findings,
                format!("[[{kind}]].{name}"),
                None,
                "present".to_owned(),
                msg.clone(),
                attribution,
            );
            let _ = ensure_entry(doc, kind, name);
        }
        ResolvedTargetTableAssertion::Absent(msg) => {
            let Some(index) = entry_index(doc, kind, name) else {
                return;
            };
            util::push_mismatch(
                findings,
                format!("[[{kind}]].{name}"),
                Some("present".to_owned()),
                "absent".to_owned(),
                msg.clone(),
                attribution,
            );
            if let Some(aot) = doc.get_mut(kind).and_then(Item::as_array_of_tables_mut) {
                let _ = aot.remove(index);
            }
        }
        ResolvedTargetTableAssertion::Fields(map) => {
            if !exists && assertion.on_empty() == OnEmpty::ChecksOnly {
                // Cannot be satisfied by writing: report the missing entry only.
                util::push_mismatch(
                    findings,
                    format!("[[{kind}]].{name}"),
                    None,
                    "present (check-only fields)".to_owned(),
                    String::new(),
                    attribution,
                );
                return;
            }
            if !exists {
                util::push_mismatch(
                    findings,
                    format!("[[{kind}]].{name}"),
                    None,
                    "present".to_owned(),
                    String::new(),
                    attribution,
                );
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
        util::attribution(resolved)
    } else {
        filtered
    }
}

fn field_assertion_fails(current: Option<&Item>, assertion: &TargetFieldAssertion) -> bool {
    match assertion {
        TargetFieldAssertion::Equals(want, _) => {
            !current.is_some_and(|item| util::scalar_matches(item, want))
        }
        TargetFieldAssertion::OneOf(allowed, _) => !current
            .and_then(Item::as_str)
            .is_some_and(|value| allowed.contains(value)),
        TargetFieldAssertion::List(_) => false,
        TargetFieldAssertion::Present(_) => current.is_none(),
        TargetFieldAssertion::Absent(_) => current.is_some(),
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
        ResolvedTargetFieldAssertion::Equals(want, msg) => {
            if read_field(doc, loc, field).is_some_and(|i| util::scalar_matches(i, want)) {
                return;
            }
            let current = read_field(doc, loc, field).and_then(util::render_item);
            util::push_mismatch(
                findings,
                path,
                current,
                util::render_scalar(want),
                msg.clone(),
                attribution,
            );
            ensure_loc(doc, loc)[field] = util::scalar_item(want);
        }
        ResolvedTargetFieldAssertion::OneOf(allowed, msg) => {
            let current = read_field(doc, loc, field).and_then(Item::as_str);
            if current.is_some_and(|c| allowed.contains(c)) {
                return;
            }
            let rendered = current.map(ToOwned::to_owned);
            util::push_mismatch(
                findings,
                path,
                rendered,
                format!("one of {allowed:?}"),
                msg.clone(),
                attribution,
            );
        }
        ResolvedTargetFieldAssertion::List(list) => {
            for (item, entry) in &list.contains {
                let item_attribution = util::attribution(entry);
                let msg = entry
                    .collected
                    .first()
                    .map(|(_, msg)| msg.as_str())
                    .unwrap_or_default();
                apply_list_contains(
                    doc,
                    loc,
                    field,
                    path.clone(),
                    core::slice::from_ref(item),
                    msg,
                    &item_attribution,
                    findings,
                );
            }
            if let Some(exact) = &list.exact {
                let exact_attribution = util::attribution(exact);
                let msg = exact
                    .collected
                    .first()
                    .map(|(_, (_, msg))| msg.as_str())
                    .unwrap_or_default();
                apply_list_is_exactly(
                    doc,
                    loc,
                    field,
                    path.clone(),
                    &exact.merged,
                    msg,
                    &exact_attribution,
                    findings,
                );
            }
            for (item, entry) in &list.excludes {
                let on_disk = read_field_array(doc, loc, field);
                let blocked = BTreeSet::from([item.clone()]);
                let present = on_disk
                    .iter()
                    .filter(|item| blocked.contains(*item))
                    .cloned()
                    .collect::<Vec<_>>();
                if !present.is_empty() {
                    let item_attribution = util::attribution(entry);
                    let msg = entry
                        .collected
                        .first()
                        .map(|(_, msg)| msg.as_str())
                        .unwrap_or_default();
                    util::push_mismatch(
                        findings,
                        path.clone(),
                        Some(format!("{on_disk:?}")),
                        format!("excludes {present:?}"),
                        msg.to_owned(),
                        &item_attribution,
                    );
                    let kept = on_disk
                        .into_iter()
                        .filter(|item| !blocked.contains(item))
                        .collect::<Vec<_>>();
                    util::write_string_array(ensure_loc(doc, loc), field, &kept);
                }
            }
        }
        ResolvedTargetFieldAssertion::Present(msg) => {
            if read_field(doc, loc, field).is_some() {
                return;
            }
            util::push_mismatch(
                findings,
                path,
                None,
                "any value (Present)".to_owned(),
                msg.clone(),
                attribution,
            );
        }
        ResolvedTargetFieldAssertion::Absent(msg) => {
            let Some(current) = read_field(doc, loc, field).and_then(util::render_item) else {
                return;
            };
            util::push_mismatch(
                findings,
                path,
                Some(current),
                "absent".to_owned(),
                msg.clone(),
                attribution,
            );
            let _ = ensure_loc(doc, loc).remove(field);
        }
    }
}

/// Read the current item for `field` at `loc`, if any.
fn read_field<'a>(doc: &'a DocumentMut, loc: &FieldLoc<'_>, field: &str) -> Option<&'a Item> {
    match loc {
        FieldLoc::Lib => util::table_ref(doc, "lib")?.get(field),
        FieldLoc::Entry { kind, name } => entry_ref(doc, kind, name)?.get(field),
    }
}

/// Read the current string array for `field` at `loc` (empty when absent).
fn read_field_array(doc: &DocumentMut, loc: &FieldLoc<'_>, field: &str) -> Vec<String> {
    match loc {
        FieldLoc::Lib => {
            util::table_ref(doc, "lib").map_or_else(Vec::new, |t| util::read_string_array(t, field))
        }
        FieldLoc::Entry { kind, name } => {
            entry_ref(doc, kind, name).map_or_else(Vec::new, |t| util::read_string_array(t, field))
        }
    }
}

/// The mutable table at `loc`, created lazily (only called on a write).
fn ensure_loc<'a>(doc: &'a mut DocumentMut, loc: &FieldLoc<'_>) -> &'a mut Table {
    match loc {
        FieldLoc::Lib => util::ensure_table(doc, "lib"),
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
    let aot = util::ensure_array_of_tables(doc, kind);
    let index = index.unwrap_or_else(|| {
        let mut t = Table::new();
        let _ = t.insert("name", value(name.to_owned()));
        aot.push(t);
        aot.len().saturating_sub(1)
    });
    aot.get_mut(index)
        .expect("index points at an existing or just-pushed entry")
}

/// `list contains`: insert the missing elements (order kept).
#[expect(
    clippy::too_many_arguments,
    reason = "thin extraction from `apply_field`'s match arm; the parameters are that arm's locals"
)]
fn apply_list_contains(
    doc: &mut DocumentMut,
    loc: &FieldLoc<'_>,
    field: &str,
    path: String,
    items: &[String],
    msg: &str,
    attribution: &[Provenance],
    findings: &mut Vec<Finding>,
) {
    let on_disk = read_field_array(doc, loc, field);
    let missing: Vec<&String> = items.iter().filter(|i| !on_disk.contains(i)).collect();
    if missing.is_empty() {
        return;
    }
    util::push_mismatch(
        findings,
        path,
        Some(format!("{on_disk:?}")),
        format!("contains {missing:?}"),
        msg.to_owned(),
        attribution,
    );
    let mut merged = on_disk;
    for item in items {
        if !merged.contains(item) {
            merged.push(item.clone());
        }
    }
    util::write_string_array(ensure_loc(doc, loc), field, &merged);
}

/// `list exact`: the array equals exactly the asserted list.
#[expect(
    clippy::too_many_arguments,
    reason = "thin extraction from `apply_field`'s match arm; the parameters are that arm's locals"
)]
fn apply_list_is_exactly(
    doc: &mut DocumentMut,
    loc: &FieldLoc<'_>,
    field: &str,
    path: String,
    items: &[String],
    msg: &str,
    attribution: &[Provenance],
    findings: &mut Vec<Finding>,
) {
    let on_disk = read_field_array(doc, loc, field);
    if on_disk == items {
        return;
    }
    util::push_mismatch(
        findings,
        path,
        Some(format!("{on_disk:?}")),
        format!("{items:?}"),
        msg.to_owned(),
        attribution,
    );
    util::write_string_array(ensure_loc(doc, loc), field, items);
}
