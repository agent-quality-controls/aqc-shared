//! Reconcile direct `[workspace].<key>` assertions (resolver, members, ...).
//!
//! Lazy: check-only assertions and vacuous removals never create `[workspace]`.

use std::collections::{BTreeMap, BTreeSet};

use aqc_file_engine_core as core_types;
use toml_edit as toml;

use crate::{reconcile::util, requirement as req};

/// The finding-path prefix for direct workspace keys.
const PREFIX: &str = "workspace";

/// Apply every direct `[workspace].<key>` requirement.
pub(crate) fn apply(
    doc: &mut toml::DocumentMut,
    merged_by_key: &BTreeMap<
        String,
        core_types::ResolvedRequirement<
            req::ResolvedWorkspaceFieldAssertion,
            req::WorkspaceFieldAssertion,
        >,
    >,
    findings: &mut Vec<core_types::Finding>,
) {
    for (key, merged) in merged_by_key {
        let attribution = attribution_for(doc, key, merged);
        apply_one(doc, key, &merged.merged, &attribution, findings);
    }
}

/// Read the on-disk item for `key` under `[workspace]`, if present.
fn current_item<'a>(doc: &'a toml::DocumentMut, key: &str) -> Option<&'a toml::Item> {
    util::table_ref(doc, PREFIX).and_then(|t| t.get(key))
}

/// The mutable `[workspace]` table if it already exists (removals only).
fn workspace_mut_existing(doc: &mut toml::DocumentMut) -> Option<&mut toml::Table> {
    doc.get_mut(PREFIX).and_then(toml::Item::as_table_mut)
}

fn attribution_for(
    doc: &toml::DocumentMut,
    key: &str,
    resolved: &core_types::ResolvedRequirement<
        req::ResolvedWorkspaceFieldAssertion,
        req::WorkspaceFieldAssertion,
    >,
) -> Vec<core_types::Provenance> {
    let current = current_item(doc, key);
    let filtered = resolved
        .collected
        .iter()
        .filter(|(_, assertion)| assertion_fails(current, assertion))
        .map(|(prov, _)| prov.clone())
        .collect::<Vec<_>>();
    if filtered.is_empty() {
        util::attribution(resolved)
    } else {
        filtered
    }
}

fn assertion_fails(current: Option<&toml::Item>, assertion: &req::WorkspaceFieldAssertion) -> bool {
    match assertion {
        req::WorkspaceFieldAssertion::Equals(want, _) => {
            !current.is_some_and(|item| util::scalar_matches(item, want))
        }
        req::WorkspaceFieldAssertion::OneOf(allowed, _) => !current
            .and_then(toml::Item::as_str)
            .is_some_and(|value| allowed.contains(value)),
        req::WorkspaceFieldAssertion::List(_) => false,
        req::WorkspaceFieldAssertion::Present(_) => current.is_none(),
        req::WorkspaceFieldAssertion::Absent(_) => current.is_some(),
    }
}

/// Apply a single resolved workspace field assertion.
fn apply_one(
    doc: &mut toml::DocumentMut,
    key: &str,
    assertion: &req::ResolvedWorkspaceFieldAssertion,
    attribution: &[core_types::Provenance],
    findings: &mut Vec<core_types::Finding>,
) {
    match assertion {
        req::ResolvedWorkspaceFieldAssertion::Equals(want, msg) => {
            apply_equals(doc, key, want, msg, attribution, findings);
        }
        req::ResolvedWorkspaceFieldAssertion::OneOf(allowed, msg) => {
            apply_one_of(doc, key, allowed, msg, attribution, findings);
        }
        req::ResolvedWorkspaceFieldAssertion::List(list) => {
            for (item, entry) in &list.contains {
                let item_attribution = util::attribution(entry);
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
                let item_attribution = util::attribution(entry);
                let msg = entry
                    .collected
                    .first()
                    .map(|(_, msg)| msg.as_str())
                    .unwrap_or_default();
                let excluded = BTreeSet::from([item.clone()]);
                apply_list_excludes(doc, key, &excluded, msg, &item_attribution, findings);
            }
            if let Some(exact) = &list.exact {
                let exact_attribution = util::attribution(exact);
                let msg = exact
                    .collected
                    .first()
                    .map(|(_, (_, msg))| msg.as_str())
                    .unwrap_or_default();
                apply_list_is_exactly(doc, key, &exact.merged, msg, &exact_attribution, findings);
            }
        }
        req::ResolvedWorkspaceFieldAssertion::Present(msg) => {
            apply_present(doc, key, msg, attribution, findings);
        }
        req::ResolvedWorkspaceFieldAssertion::Absent(msg) => {
            apply_absent(doc, key, msg, attribution, findings);
        }
    }
}

/// `key == want`.
fn apply_equals(
    doc: &mut toml::DocumentMut,
    key: &str,
    want: &core_types::ConfigScalar,
    msg: &str,
    attribution: &[core_types::Provenance],
    findings: &mut Vec<core_types::Finding>,
) {
    if current_item(doc, key).is_some_and(|it| util::scalar_matches(it, want)) {
        return;
    }
    let current = current_item(doc, key).and_then(util::render_item);
    util::push_mismatch(
        findings,
        format!("[{PREFIX}].{key}"),
        current,
        util::render_scalar(want),
        msg.to_owned(),
        attribution,
    );
    util::ensure_table(doc, PREFIX)[key] = util::scalar_item(want);
}

/// `key` is one of `allowed` (check-only).
fn apply_one_of(
    doc: &toml::DocumentMut,
    key: &str,
    allowed: &BTreeSet<String>,
    msg: &str,
    attribution: &[core_types::Provenance],
    findings: &mut Vec<core_types::Finding>,
) {
    let current = current_item(doc, key).and_then(toml::Item::as_str);
    if current.is_some_and(|c| allowed.contains(c)) {
        return;
    }
    util::push_mismatch(
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
    doc: &mut toml::DocumentMut,
    key: &str,
    items: &[String],
    msg: &str,
    attribution: &[core_types::Provenance],
    findings: &mut Vec<core_types::Finding>,
) {
    let on_disk =
        util::table_ref(doc, PREFIX).map_or_else(Vec::new, |t| util::read_string_array(t, key));
    let on_disk_set: BTreeSet<&str> = on_disk.iter().map(String::as_str).collect();
    let missing: Vec<&String> = items
        .iter()
        .filter(|w| !on_disk_set.contains(w.as_str()))
        .collect();
    if missing.is_empty() {
        return;
    }
    util::push_mismatch(
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
    util::write_string_array(util::ensure_table(doc, PREFIX), key, &new_list);
}

/// The on-disk list contains none of these elements (vacuous when absent).
fn apply_list_excludes(
    doc: &mut toml::DocumentMut,
    key: &str,
    items: &BTreeSet<String>,
    msg: &str,
    attribution: &[core_types::Provenance],
    findings: &mut Vec<core_types::Finding>,
) {
    let on_disk =
        util::table_ref(doc, PREFIX).map_or_else(Vec::new, |t| util::read_string_array(t, key));
    let present: Vec<&String> = items.iter().filter(|x| on_disk.contains(x)).collect();
    if present.is_empty() {
        return;
    }
    util::push_mismatch(
        findings,
        format!("[{PREFIX}].{key}"),
        Some(format!("{on_disk:?}")),
        format!("excludes {present:?}"),
        msg.to_owned(),
        attribution,
    );
    let new_list: Vec<String> = on_disk.into_iter().filter(|e| !items.contains(e)).collect();
    util::write_string_array(util::ensure_table(doc, PREFIX), key, &new_list);
}

/// The on-disk list equals exactly `items`.
fn apply_list_is_exactly(
    doc: &mut toml::DocumentMut,
    key: &str,
    items: &[String],
    msg: &str,
    attribution: &[core_types::Provenance],
    findings: &mut Vec<core_types::Finding>,
) {
    let on_disk =
        util::table_ref(doc, PREFIX).map_or_else(Vec::new, |t| util::read_string_array(t, key));
    if on_disk == items {
        return;
    }
    util::push_mismatch(
        findings,
        format!("[{PREFIX}].{key}"),
        Some(format!("{on_disk:?}")),
        format!("{items:?}"),
        msg.to_owned(),
        attribution,
    );
    util::write_string_array(util::ensure_table(doc, PREFIX), key, items);
}

/// `key` is set, to anything (check-only).
fn apply_present(
    doc: &toml::DocumentMut,
    key: &str,
    msg: &str,
    attribution: &[core_types::Provenance],
    findings: &mut Vec<core_types::Finding>,
) {
    if current_item(doc, key).is_some() {
        return;
    }
    util::push_mismatch(
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
    doc: &mut toml::DocumentMut,
    key: &str,
    msg: &str,
    attribution: &[core_types::Provenance],
    findings: &mut Vec<core_types::Finding>,
) {
    let current = current_item(doc, key).and_then(util::render_item);
    if current_item(doc, key).is_none() {
        return;
    }
    util::push_mismatch(
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
