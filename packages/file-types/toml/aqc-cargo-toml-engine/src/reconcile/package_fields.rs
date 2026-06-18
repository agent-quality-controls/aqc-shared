//! Reconcile `[package].<field>` (and `[workspace.package].<field>`).
//!
//! Lazy: a check-only assertion (`OneOf`, `Present`) and a vacuous removal
//! (`Absent` or a list exclusion on an absent key) never create the table. The
//! table is fetched mutably only when a write is about to happen.

#![cfg_attr(
    not(test),
    expect(
        clippy::missing_docs_in_private_items,
        reason = "Private package-field helpers are internal reconciliation steps."
    )
)]
#![expect(
    clippy::type_complexity,
    reason = "Package-field reconciliation consumes resolved requirement map shapes."
)]

use std::collections::{BTreeMap, BTreeSet};

use aqc_file_engine_core::{ConfigScalar, Finding, Provenance, ResolvedRequirement};
use toml_edit::{DocumentMut, Item, Table};

use crate::reconcile::util;
use crate::requirement::{PackageFieldAssertion, ResolvedPackageFieldAssertion};

/// Whether the fields target `[package]` or `[workspace.package]`. The latter
/// is the inheritance source, so `InheritsWorkspace` there is a schema error.
#[derive(Clone, Copy)]
pub(crate) enum PackageScope {
    /// `[package]`.
    Package,
    /// `[workspace.package]`.
    WorkspacePackage,
}

impl PackageScope {
    /// The finding-path prefix for this scope (`package` / `workspace.package`).
    const fn prefix(self) -> &'static str {
        match self {
            Self::Package => "package",
            Self::WorkspacePackage => "workspace.package",
        }
    }

    /// True when this scope is the workspace inheritance source.
    const fn is_workspace_source(self) -> bool {
        matches!(self, Self::WorkspacePackage)
    }
}

/// Apply every `[package].<field>` requirement at the given scope.
pub(crate) fn apply(
    doc: &mut DocumentMut,
    scope: PackageScope,
    merged_by_field: &BTreeMap<
        String,
        ResolvedRequirement<
            ResolvedPackageFieldAssertion,
            crate::requirement::PackageFieldAssertion,
        >,
    >,
    findings: &mut Vec<Finding>,
) {
    for (field, merged) in merged_by_field {
        let attribution = attribution_for(doc, scope, field, merged);
        apply_one(doc, scope, field, &merged.merged, &attribution, findings);
    }
}

/// The mutable target table for the scope, creating it (and any parent) lazily.
fn ensure_target(doc: &mut DocumentMut, scope: PackageScope) -> &mut Table {
    match scope {
        PackageScope::Package => util::ensure_table(doc, "package"),
        PackageScope::WorkspacePackage => {
            let ws = util::ensure_table(doc, "workspace");
            util::ensure_nested(ws, "package")
        }
    }
}

/// The read-only target table for the scope, if it exists.
fn target_ref(doc: &DocumentMut, scope: PackageScope) -> Option<&Table> {
    match scope {
        PackageScope::Package => util::table_ref(doc, "package"),
        PackageScope::WorkspacePackage => util::table_ref(doc, "workspace")
            .and_then(|ws| ws.get("package").and_then(Item::as_table)),
    }
}

fn attribution_for(
    doc: &DocumentMut,
    scope: PackageScope,
    field: &str,
    resolved: &ResolvedRequirement<ResolvedPackageFieldAssertion, PackageFieldAssertion>,
) -> Vec<Provenance> {
    let current = current_item(doc, scope, field);
    let filtered = resolved
        .collected
        .iter()
        .filter(|(_, assertion)| assertion_fails(scope, current, assertion))
        .map(|(prov, _)| prov.clone())
        .collect::<Vec<_>>();
    if filtered.is_empty() {
        util::attribution(resolved)
    } else {
        filtered
    }
}

fn assertion_fails(
    scope: PackageScope,
    current: Option<&Item>,
    assertion: &PackageFieldAssertion,
) -> bool {
    match assertion {
        PackageFieldAssertion::Equals(want, _) => {
            !current.is_some_and(|item| util::scalar_matches(item, want))
        }
        PackageFieldAssertion::AtLeastVersion(min, _) => !current
            .and_then(Item::as_str)
            .is_some_and(|value| util::ge_version(value, min)),
        PackageFieldAssertion::OneOf(allowed, _) => !current
            .and_then(Item::as_str)
            .is_some_and(|value| allowed.contains(value)),
        PackageFieldAssertion::List(_) => false,
        PackageFieldAssertion::InheritsWorkspace(_) => {
            scope.is_workspace_source() || !current.is_some_and(util::is_workspace_inherit)
        }
        PackageFieldAssertion::Present(_) => current.is_none(),
        PackageFieldAssertion::Absent(_) => current.is_some(),
    }
}

/// Read the on-disk item for `field` at this scope, if present.
fn current_item<'a>(doc: &'a DocumentMut, scope: PackageScope, field: &str) -> Option<&'a Item> {
    target_ref(doc, scope).and_then(|t| t.get(field))
}

/// Apply a single resolved package field assertion.
fn apply_one(
    doc: &mut DocumentMut,
    scope: PackageScope,
    field: &str,
    assertion: &ResolvedPackageFieldAssertion,
    attribution: &[Provenance],
    findings: &mut Vec<Finding>,
) {
    match assertion {
        ResolvedPackageFieldAssertion::Equals(want, msg) => {
            apply_equals(doc, scope, field, want, msg, attribution, findings);
        }
        ResolvedPackageFieldAssertion::AtLeastVersion(min, msg) => {
            apply_at_least(doc, scope, field, min, msg, attribution, findings);
        }
        ResolvedPackageFieldAssertion::OneOf(allowed, msg) => {
            apply_one_of(doc, scope, field, allowed, msg, attribution, findings);
        }
        ResolvedPackageFieldAssertion::List(list) => {
            for (item, entry) in &list.contains {
                let item_attribution = util::attribution(entry);
                let msg = entry
                    .collected
                    .first()
                    .map(|(_, msg)| msg.as_str())
                    .unwrap_or_default();
                apply_list_contains(
                    doc,
                    scope,
                    field,
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
                apply_list_excludes(
                    doc,
                    scope,
                    field,
                    &excluded,
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
                    scope,
                    field,
                    &exact.merged,
                    msg,
                    &exact_attribution,
                    findings,
                );
            }
        }
        ResolvedPackageFieldAssertion::InheritsWorkspace(msg) => {
            apply_inherits(doc, scope, field, msg, attribution, findings);
        }
        ResolvedPackageFieldAssertion::Present(msg) => {
            apply_present(doc, scope, field, msg, attribution, findings);
        }
        ResolvedPackageFieldAssertion::Absent(msg) => {
            apply_absent(doc, scope, field, msg, attribution, findings);
        }
    }
}

/// `field == want` (string/int/bool form).
fn apply_equals(
    doc: &mut DocumentMut,
    scope: PackageScope,
    field: &str,
    want: &ConfigScalar,
    msg: &str,
    attribution: &[Provenance],
    findings: &mut Vec<Finding>,
) {
    if current_item(doc, scope, field).is_some_and(|it| util::scalar_matches(it, want)) {
        return;
    }
    let current = current_item(doc, scope, field).and_then(util::render_item);
    util::push_mismatch(
        findings,
        format!("[{}].{field}", scope.prefix()),
        current,
        util::render_scalar(want),
        msg.to_owned(),
        attribution,
    );
    ensure_target(doc, scope)[field] = util::scalar_item(want);
}

/// `field >= min` (version ordering; works for editions).
fn apply_at_least(
    doc: &mut DocumentMut,
    scope: PackageScope,
    field: &str,
    min: &str,
    msg: &str,
    attribution: &[Provenance],
    findings: &mut Vec<Finding>,
) {
    let current = current_item(doc, scope, field).and_then(Item::as_str);
    if current.is_some_and(|c| util::ge_version(c, min)) {
        return;
    }
    util::push_mismatch(
        findings,
        format!("[{}].{field}", scope.prefix()),
        current.map(ToOwned::to_owned),
        format!("at least {min}"),
        msg.to_owned(),
        attribution,
    );
    ensure_target(doc, scope)[field] = util::scalar_item(&ConfigScalar::Str(min.to_owned()));
}

/// `field` is one of `allowed` (check-only).
fn apply_one_of(
    doc: &DocumentMut,
    scope: PackageScope,
    field: &str,
    allowed: &BTreeSet<String>,
    msg: &str,
    attribution: &[Provenance],
    findings: &mut Vec<Finding>,
) {
    let current = current_item(doc, scope, field).and_then(Item::as_str);
    if current.is_some_and(|c| allowed.contains(c)) {
        return;
    }
    util::push_mismatch(
        findings,
        format!("[{}].{field}", scope.prefix()),
        current.map(ToOwned::to_owned),
        format!("one of {allowed:?}"),
        msg.to_owned(),
        attribution,
    );
}

/// The on-disk list contains every requested element.
fn apply_list_contains(
    doc: &mut DocumentMut,
    scope: PackageScope,
    field: &str,
    items: &[String],
    msg: &str,
    attribution: &[Provenance],
    findings: &mut Vec<Finding>,
) {
    let on_disk =
        target_ref(doc, scope).map_or_else(Vec::new, |t| util::read_string_array(t, field));
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
        format!("[{}].{field}", scope.prefix()),
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
    util::write_string_array(ensure_target(doc, scope), field, &new_list);
}

/// The on-disk list contains none of these elements (vacuous when absent).
fn apply_list_excludes(
    doc: &mut DocumentMut,
    scope: PackageScope,
    field: &str,
    items: &BTreeSet<String>,
    msg: &str,
    attribution: &[Provenance],
    findings: &mut Vec<Finding>,
) {
    let on_disk =
        target_ref(doc, scope).map_or_else(Vec::new, |t| util::read_string_array(t, field));
    let present: Vec<&String> = items.iter().filter(|x| on_disk.contains(x)).collect();
    if present.is_empty() {
        return;
    }
    util::push_mismatch(
        findings,
        format!("[{}].{field}", scope.prefix()),
        Some(format!("{on_disk:?}")),
        format!("excludes {present:?}"),
        msg.to_owned(),
        attribution,
    );
    let new_list: Vec<String> = on_disk.into_iter().filter(|e| !items.contains(e)).collect();
    util::write_string_array(ensure_target(doc, scope), field, &new_list);
}

/// The on-disk list equals exactly `items`.
fn apply_list_is_exactly(
    doc: &mut DocumentMut,
    scope: PackageScope,
    field: &str,
    items: &[String],
    msg: &str,
    attribution: &[Provenance],
    findings: &mut Vec<Finding>,
) {
    let on_disk =
        target_ref(doc, scope).map_or_else(Vec::new, |t| util::read_string_array(t, field));
    if on_disk == items {
        return;
    }
    util::push_mismatch(
        findings,
        format!("[{}].{field}", scope.prefix()),
        Some(format!("{on_disk:?}")),
        format!("{items:?}"),
        msg.to_owned(),
        attribution,
    );
    util::write_string_array(ensure_target(doc, scope), field, items);
}

/// The field uses workspace inheritance: `<field> = { workspace = true }`.
/// In `[workspace.package]` this is invalid (the source can't inherit itself).
fn apply_inherits(
    doc: &mut DocumentMut,
    scope: PackageScope,
    field: &str,
    msg: &str,
    attribution: &[Provenance],
    findings: &mut Vec<Finding>,
) {
    if scope.is_workspace_source() {
        findings.push(Finding::InvalidRequirements {
            key: format!("[{}].{field}", scope.prefix()),
            message: format!(
                "InheritsWorkspace is invalid in [workspace.package].{field}: this table is the inheritance source. {msg}"
            ),
            contributors: attribution
                .iter()
                .map(|p| (p.policy.clone(), "InheritsWorkspace".to_owned()))
                .collect(),
        });
        return;
    }
    if current_item(doc, scope, field).is_some_and(util::is_workspace_inherit) {
        return;
    }
    let current = current_item(doc, scope, field).and_then(util::render_item);
    util::push_mismatch(
        findings,
        format!("[{}].{field}", scope.prefix()),
        current,
        "{ workspace = true }".to_owned(),
        msg.to_owned(),
        attribution,
    );
    ensure_target(doc, scope)[field] = util::workspace_inline();
}

/// The field is set, to anything (check-only).
fn apply_present(
    doc: &DocumentMut,
    scope: PackageScope,
    field: &str,
    msg: &str,
    attribution: &[Provenance],
    findings: &mut Vec<Finding>,
) {
    if current_item(doc, scope, field).is_some() {
        return;
    }
    util::push_mismatch(
        findings,
        format!("[{}].{field}", scope.prefix()),
        None,
        "any value (Present)".to_owned(),
        msg.to_owned(),
        attribution,
    );
}

/// The field is not set (vacuous when already absent).
fn apply_absent(
    doc: &mut DocumentMut,
    scope: PackageScope,
    field: &str,
    msg: &str,
    attribution: &[Provenance],
    findings: &mut Vec<Finding>,
) {
    let current = current_item(doc, scope, field).and_then(util::render_item);
    if current_item(doc, scope, field).is_none() {
        return;
    }
    util::push_mismatch(
        findings,
        format!("[{}].{field}", scope.prefix()),
        current,
        "absent".to_owned(),
        msg.to_owned(),
        attribution,
    );
    if let Some(t) = target_mut_existing(doc, scope) {
        let _ = t.remove(field);
    }
}

/// The mutable target table if it already exists (removals must not create it).
fn target_mut_existing(doc: &mut DocumentMut, scope: PackageScope) -> Option<&mut Table> {
    match scope {
        PackageScope::Package => doc.get_mut("package").and_then(Item::as_table_mut),
        PackageScope::WorkspacePackage => doc
            .get_mut("workspace")
            .and_then(Item::as_table_mut)
            .and_then(|ws| ws.get_mut("package").and_then(Item::as_table_mut)),
    }
}
