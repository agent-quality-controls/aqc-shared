//! Reconcile direct `[workspace].<key>` assertions (resolver, members, ...).
//!
//! Lazy: check-only assertions and vacuous removals never create `[workspace]`.

use std::collections::{BTreeMap, BTreeSet};

use aqc_file_engine_core::{ConfigScalar, Finding, Provenance, ResolvedRequirement};
use toml_edit::{DocumentMut, Item, Table};

use crate::reconcile::util::{
    attribution as resolved_attribution, ensure_table, push_mismatch, read_string_array,
    render_item, render_scalar, scalar_item, scalar_matches, table_ref, write_string_array,
};
use crate::requirement::{ResolvedWorkspaceFieldAssertion, WorkspaceFieldAssertion};

/// The finding-path prefix for direct workspace keys.
const PREFIX: &str = "workspace";

/// Apply every direct `[workspace].<key>` requirement.
pub(crate) fn apply(
    doc: &mut DocumentMut,
    merged_by_key: &BTreeMap<
        String,
        ResolvedRequirement<
            ResolvedWorkspaceFieldAssertion,
            crate::requirement::WorkspaceFieldAssertion,
        >,
    >,
    findings: &mut Vec<Finding>,
) {
    for (key, merged) in merged_by_key {
        let attribution = attribution_for(doc, key, merged);
        apply_one(doc, key, &merged.merged, &attribution, findings);
    }
}

/// Read the on-disk item for `key` under `[workspace]`, if present.
fn current_item<'a>(doc: &'a DocumentMut, key: &str) -> Option<&'a Item> {
    table_ref(doc, PREFIX).and_then(|t| t.get(key))
}

/// The mutable `[workspace]` table if it already exists (removals only).
fn workspace_mut_existing(doc: &mut DocumentMut) -> Option<&mut Table> {
    doc.get_mut(PREFIX).and_then(Item::as_table_mut)
}

fn attribution_for(
    doc: &DocumentMut,
    key: &str,
    resolved: &ResolvedRequirement<ResolvedWorkspaceFieldAssertion, WorkspaceFieldAssertion>,
) -> Vec<Provenance> {
    let current = current_item(doc, key);
    let filtered = resolved
        .collected
        .iter()
        .filter(|(_, assertion)| assertion_fails(current, assertion))
        .map(|(prov, _)| prov.clone())
        .collect::<Vec<_>>();
    if filtered.is_empty() {
        resolved_attribution(resolved)
    } else {
        filtered
    }
}

fn assertion_fails(current: Option<&Item>, assertion: &WorkspaceFieldAssertion) -> bool {
    match assertion {
        WorkspaceFieldAssertion::Equals(want, _) => {
            !current.is_some_and(|item| scalar_matches(item, want))
        }
        WorkspaceFieldAssertion::OneOf(allowed, _) => !current
            .and_then(Item::as_str)
            .is_some_and(|value| allowed.contains(value)),
        WorkspaceFieldAssertion::List(_) => false,
        WorkspaceFieldAssertion::Present(_) => current.is_none(),
        WorkspaceFieldAssertion::Absent(_) => current.is_some(),
    }
}

/// Apply a single resolved workspace field assertion.
fn apply_one(
    doc: &mut DocumentMut,
    key: &str,
    assertion: &ResolvedWorkspaceFieldAssertion,
    attribution: &[Provenance],
    findings: &mut Vec<Finding>,
) {
    match assertion {
        ResolvedWorkspaceFieldAssertion::Equals(want, msg) => {
            apply_equals(doc, key, want, msg, attribution, findings);
        }
        ResolvedWorkspaceFieldAssertion::OneOf(allowed, msg) => {
            apply_one_of(doc, key, allowed, msg, attribution, findings);
        }
        ResolvedWorkspaceFieldAssertion::List(list) => {
            for (item, entry) in &list.contains {
                let item_attribution = resolved_attribution(entry);
                let msg = entry
                    .collected
                    .first()
                    .map(|(_, msg)| msg.as_str())
                    .unwrap_or_default();
                apply_list_contains(
                    doc,
                    key,
                    core::slice::from_ref(item),
                    msg,
                    &item_attribution,
                    findings,
                );
            }
            for (item, entry) in &list.excludes {
                let item_attribution = resolved_attribution(entry);
                let msg = entry
                    .collected
                    .first()
                    .map(|(_, msg)| msg.as_str())
                    .unwrap_or_default();
                let excluded = BTreeSet::from([item.clone()]);
                apply_list_excludes(doc, key, &excluded, msg, &item_attribution, findings);
            }
            if let Some(exact) = &list.exact {
                let exact_attribution = resolved_attribution(exact);
                let msg = exact
                    .collected
                    .first()
                    .map(|(_, (_, msg))| msg.as_str())
                    .unwrap_or_default();
                apply_list_is_exactly(doc, key, &exact.merged, msg, &exact_attribution, findings);
            }
        }
        ResolvedWorkspaceFieldAssertion::Present(msg) => {
            apply_present(doc, key, msg, attribution, findings);
        }
        ResolvedWorkspaceFieldAssertion::Absent(msg) => {
            apply_absent(doc, key, msg, attribution, findings);
        }
    }
}

/// `key == want`.
fn apply_equals(
    doc: &mut DocumentMut,
    key: &str,
    want: &ConfigScalar,
    msg: &str,
    attribution: &[Provenance],
    findings: &mut Vec<Finding>,
) {
    if current_item(doc, key).is_some_and(|it| scalar_matches(it, want)) {
        return;
    }
    let current = current_item(doc, key).and_then(render_item);
    push_mismatch(
        findings,
        format!("[{PREFIX}].{key}"),
        current,
        render_scalar(want),
        msg.to_owned(),
        attribution,
    );
    ensure_table(doc, PREFIX)[key] = scalar_item(want);
}

/// `key` is one of `allowed` (check-only).
fn apply_one_of(
    doc: &DocumentMut,
    key: &str,
    allowed: &BTreeSet<String>,
    msg: &str,
    attribution: &[Provenance],
    findings: &mut Vec<Finding>,
) {
    let current = current_item(doc, key).and_then(Item::as_str);
    if current.is_some_and(|c| allowed.contains(c)) {
        return;
    }
    push_mismatch(
        findings,
        format!("[{PREFIX}].{key}"),
        current.map(ToOwned::to_owned),
        format!("one of {allowed:?}"),
        msg.to_owned(),
        attribution,
    );
}

/// The on-disk list contains every requested element.
fn apply_list_contains(
    doc: &mut DocumentMut,
    key: &str,
    items: &[String],
    msg: &str,
    attribution: &[Provenance],
    findings: &mut Vec<Finding>,
) {
    let on_disk = table_ref(doc, PREFIX).map_or_else(Vec::new, |t| read_string_array(t, key));
    let on_disk_set: BTreeSet<&str> = on_disk.iter().map(String::as_str).collect();
    let missing: Vec<&String> = items
        .iter()
        .filter(|w| !on_disk_set.contains(w.as_str()))
        .collect();
    if missing.is_empty() {
        return;
    }
    push_mismatch(
        findings,
        format!("[{PREFIX}].{key}"),
        Some(format!("{on_disk:?}")),
        format!("contains {missing:?}"),
        msg.to_owned(),
        attribution,
    );
    let mut new_list = on_disk;
    for w in items {
        if !new_list.iter().any(|e| e == w) {
            new_list.push(w.clone());
        }
    }
    write_string_array(ensure_table(doc, PREFIX), key, &new_list);
}

/// The on-disk list contains none of these elements (vacuous when absent).
fn apply_list_excludes(
    doc: &mut DocumentMut,
    key: &str,
    items: &BTreeSet<String>,
    msg: &str,
    attribution: &[Provenance],
    findings: &mut Vec<Finding>,
) {
    let on_disk = table_ref(doc, PREFIX).map_or_else(Vec::new, |t| read_string_array(t, key));
    let present: Vec<&String> = items.iter().filter(|x| on_disk.contains(x)).collect();
    if present.is_empty() {
        return;
    }
    push_mismatch(
        findings,
        format!("[{PREFIX}].{key}"),
        Some(format!("{on_disk:?}")),
        format!("excludes {present:?}"),
        msg.to_owned(),
        attribution,
    );
    let new_list: Vec<String> = on_disk.into_iter().filter(|e| !items.contains(e)).collect();
    write_string_array(ensure_table(doc, PREFIX), key, &new_list);
}

/// The on-disk list equals exactly `items`.
fn apply_list_is_exactly(
    doc: &mut DocumentMut,
    key: &str,
    items: &[String],
    msg: &str,
    attribution: &[Provenance],
    findings: &mut Vec<Finding>,
) {
    let on_disk = table_ref(doc, PREFIX).map_or_else(Vec::new, |t| read_string_array(t, key));
    if on_disk == items {
        return;
    }
    push_mismatch(
        findings,
        format!("[{PREFIX}].{key}"),
        Some(format!("{on_disk:?}")),
        format!("{items:?}"),
        msg.to_owned(),
        attribution,
    );
    write_string_array(ensure_table(doc, PREFIX), key, items);
}

/// `key` is set, to anything (check-only).
fn apply_present(
    doc: &DocumentMut,
    key: &str,
    msg: &str,
    attribution: &[Provenance],
    findings: &mut Vec<Finding>,
) {
    if current_item(doc, key).is_some() {
        return;
    }
    push_mismatch(
        findings,
        format!("[{PREFIX}].{key}"),
        None,
        "any value (Present)".to_owned(),
        msg.to_owned(),
        attribution,
    );
}

/// `key` is not set (vacuous when already absent).
fn apply_absent(
    doc: &mut DocumentMut,
    key: &str,
    msg: &str,
    attribution: &[Provenance],
    findings: &mut Vec<Finding>,
) {
    let current = current_item(doc, key).and_then(render_item);
    if current_item(doc, key).is_none() {
        return;
    }
    push_mismatch(
        findings,
        format!("[{PREFIX}].{key}"),
        current,
        "absent".to_owned(),
        msg.to_owned(),
        attribution,
    );
    if let Some(t) = workspace_mut_existing(doc) {
        let _ = t.remove(key);
    }
}
