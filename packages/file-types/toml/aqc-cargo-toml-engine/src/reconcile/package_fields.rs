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
#![allow(
    clippy::type_complexity,
    reason = "Package-field reconciliation consumes resolved requirement map shapes."
)]

use std::collections::{BTreeMap, BTreeSet};

use aqc_file_engine_core::{
    ConfigScalar, DottedVersion, Finding, Provenance, ResolvedRequirement, ScalarAssertion,
    Severity, parse_version_tuple,
};
use aqc_toml_engine_core::{ScalarFieldEdit, scalar_field_edit};
use toml_edit::{DocumentMut, Item, Table, TableLike};

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
        PackageScope::Package => aqc_toml_engine_core::ensure_table(doc, "package"),
        PackageScope::WorkspacePackage => {
            let ws = aqc_toml_engine_core::ensure_table(doc, "workspace");
            aqc_toml_engine_core::ensure_nested(ws, "package")
        }
    }
}

/// The read-only target table for the scope, if it exists.
fn target_ref(doc: &DocumentMut, scope: PackageScope) -> Option<&dyn TableLike> {
    match scope {
        PackageScope::Package => aqc_toml_engine_core::table_ref(doc, "package"),
        PackageScope::WorkspacePackage => aqc_toml_engine_core::table_ref(doc, "workspace")
            .and_then(|ws| ws.get("package").and_then(Item::as_table_like)),
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
        resolved.attribution()
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
        PackageFieldAssertion::Scalar(assertion) => {
            aqc_toml_engine_core::scalar_assertion_fails(current, assertion)
        }
        PackageFieldAssertion::OrderedVersion(assertion) => {
            dotted_version_fails(current.and_then(Item::as_str), assertion)
        }
        PackageFieldAssertion::List(_) => false,
        PackageFieldAssertion::InheritsWorkspace(_) => {
            scope.is_workspace_source() || !current.is_some_and(util::is_workspace_inherit)
        }
    }
}

/// Read the on-disk item for `field` at this scope, if present.
fn current_item<'a>(doc: &'a DocumentMut, scope: PackageScope, field: &str) -> Option<&'a Item> {
    target_ref(doc, scope).and_then(|t| t.get(field))
}

fn dotted_version_fails(current: Option<&str>, assertion: &ScalarAssertion<DottedVersion>) -> bool {
    match assertion {
        ScalarAssertion::Equals(want, _) => current != Some(want.as_str()),
        ScalarAssertion::AtLeast(min, _) => current
            .is_none_or(|value| parse_version_tuple(value) < parse_version_tuple(min.as_str())),
        ScalarAssertion::OneOf(allowed, _) => {
            !current.is_some_and(|value| allowed.iter().any(|allowed| allowed.as_str() == value))
        }
        ScalarAssertion::Present(_) => current.is_none(),
        ScalarAssertion::Absent(_) => current.is_some(),
        ScalarAssertion::AtMost(..) | ScalarAssertion::Range(..) => true,
    }
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
        ResolvedPackageFieldAssertion::Scalar(assertion) => {
            apply_config_scalar(doc, scope, field, assertion, attribution, findings);
        }
        ResolvedPackageFieldAssertion::OrderedVersion(assertion) => {
            apply_dotted_version(doc, scope, field, assertion, attribution, findings);
        }
        ResolvedPackageFieldAssertion::List(list) => {
            let display_key = format!("[{}].{field}", scope.prefix());
            let target = target_ref(doc, scope);
            if aqc_toml_engine_core::report_list_item_shape(
                target.and_then(|table| table.get(field)),
                &display_key,
                list,
                findings,
            ) {
                let current = target
                    .and_then(|table| {
                        aqc_toml_engine_core::table_list_values_optional(table, field)
                    })
                    .unwrap_or_default();
                let updated = aqc_file_engine_core::apply_list_requirements(&current, list);
                aqc_toml_engine_core::write_table_list(ensure_target(doc, scope), field, &updated);
                return;
            }
            let current = target
                .and_then(|table| aqc_toml_engine_core::table_list_values_optional(table, field));
            if let Some(updated) = aqc_toml_engine_core::reconcile_optional_table_list_field(
                display_key,
                current,
                list,
                findings,
            ) {
                aqc_toml_engine_core::write_table_list(ensure_target(doc, scope), field, &updated);
            }
        }
        ResolvedPackageFieldAssertion::InheritsWorkspace(msg) => {
            apply_inherits(doc, scope, field, msg, attribution, findings);
        }
    }
}

/// `field == want` (string/int/bool form).
fn apply_config_scalar(
    doc: &mut DocumentMut,
    scope: PackageScope,
    field: &str,
    assertion: &ScalarAssertion<ConfigScalar>,
    attribution: &[Provenance],
    findings: &mut Vec<Finding>,
) {
    match scalar_field_edit(
        format!("[{}].{field}", scope.prefix()),
        current_item(doc, scope, field),
        assertion,
        attribution,
        findings,
    ) {
        Some(ScalarFieldEdit::Write(item)) => {
            ensure_target(doc, scope)[field] = item;
        }
        Some(ScalarFieldEdit::Remove) => {
            if let Some(t) = target_mut_existing(doc, scope) {
                let _ = t.remove(field);
            }
        }
        None => {}
    }
}

fn apply_dotted_version(
    doc: &mut DocumentMut,
    scope: PackageScope,
    field: &str,
    assertion: &ScalarAssertion<DottedVersion>,
    attribution: &[Provenance],
    findings: &mut Vec<Finding>,
) {
    match assertion {
        ScalarAssertion::Equals(want, msg) => {
            apply_config_scalar(
                doc,
                scope,
                field,
                &ScalarAssertion::Equals(ConfigScalar::Str(want.as_str().to_owned()), msg.clone()),
                attribution,
                findings,
            );
        }
        ScalarAssertion::AtLeast(min, msg) => {
            let current = current_item(doc, scope, field).and_then(Item::as_str);
            if current.is_some_and(|c| parse_version_tuple(c) >= parse_version_tuple(min.as_str()))
            {
                return;
            }
            findings.push(Finding::Mismatch {
                key: format!("[{}].{field}", scope.prefix()),
                selector: None,
                current: current.map(ToOwned::to_owned),
                expected: format!("at least {}", min.as_str()),
                message: msg.to_owned(),
                severity: Severity::Error,
                attribution: attribution.to_vec(),
            });
            ensure_target(doc, scope)[field] =
                aqc_toml_engine_core::scalar_item(&ConfigScalar::Str(min.as_str().to_owned()));
        }
        ScalarAssertion::OneOf(allowed, msg) => {
            let current = current_item(doc, scope, field).and_then(Item::as_str);
            if current.is_some_and(|c| allowed.iter().any(|value| value.as_str() == c)) {
                return;
            }
            let allowed = allowed
                .iter()
                .map(|value| value.as_str().to_owned())
                .collect::<BTreeSet<_>>();
            findings.push(Finding::Mismatch {
                key: format!("[{}].{field}", scope.prefix()),
                selector: None,
                current: current.map(ToOwned::to_owned),
                expected: format!("one of {allowed:?}"),
                message: msg.to_owned(),
                severity: Severity::Error,
                attribution: attribution.to_vec(),
            });
        }
        ScalarAssertion::Present(msg) => apply_config_scalar(
            doc,
            scope,
            field,
            &ScalarAssertion::Present(msg.clone()),
            attribution,
            findings,
        ),
        ScalarAssertion::Absent(msg) => apply_config_scalar(
            doc,
            scope,
            field,
            &ScalarAssertion::Absent(msg.clone()),
            attribution,
            findings,
        ),
        ScalarAssertion::AtMost(..) | ScalarAssertion::Range(..) => {}
    }
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
    let current = current_item(doc, scope, field).and_then(aqc_toml_engine_core::render_item);
    findings.push(Finding::Mismatch {
        key: format!("[{}].{field}", scope.prefix()),
        selector: None,
        current,
        expected: "{ workspace = true }".to_owned(),
        message: msg.to_owned(),
        severity: Severity::Error,
        attribution: attribution.to_vec(),
    });
    ensure_target(doc, scope)[field] = util::workspace_inline();
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
