//! Reconcile `[profile.<name>]` tables, including
//! `[profile.<name>.package.<spec>]` overrides and `[profile.<name>.build-override]`.
//!
//! Lazy: check-only field assertions (`OneOf`, `Present`) and vacuous removals
//! create no tables.

#![cfg_attr(
    not(test),
    expect(
        clippy::missing_docs_in_private_items,
        reason = "Private profile reconciliation helpers are internal steps."
    )
)]

use std::collections::BTreeMap;

use aqc_file_engine_core::{
    ConfigScalar, Finding, Provenance, ResolvedRequirement, ScalarAssertion,
};
use aqc_toml_engine_core::{
    ScalarFieldEdit, attribution as resolved_attribution, ensure_table_at, scalar_field_edit,
    table_at, table_at_mut,
};
use toml_edit::{DocumentMut, Item};

use crate::requirement::ResolvedProfileRequirements;

/// Resolved profile scalar-field assertion.
type ResolvedProfileScalarField =
    ResolvedRequirement<ScalarAssertion<ConfigScalar>, ScalarAssertion<ConfigScalar>>;

/// Apply every `[profile.<name>]` requirement.
pub(crate) fn apply(
    doc: &mut DocumentMut,
    merged_by_profile: &BTreeMap<String, ResolvedProfileRequirements>,
    findings: &mut Vec<Finding>,
) {
    for (profile, requirement) in merged_by_profile {
        apply_profile(doc, profile, requirement, findings);
    }
}

/// Apply one profile requirement.
fn apply_profile(
    doc: &mut DocumentMut,
    profile: &str,
    requirement: &ResolvedProfileRequirements,
    findings: &mut Vec<Finding>,
) {
    for (field, fa) in &requirement.fields {
        let path = vec!["profile".to_owned(), profile.to_owned()];
        let display = format!("[profile.{profile}]");
        apply_resolved_field(doc, &path, &display, field, fa, findings);
    }
    if let Some(build_override) = &requirement.build_override {
        let path = vec![
            "profile".to_owned(),
            profile.to_owned(),
            "build-override".to_owned(),
        ];
        let display = format!("[profile.{profile}.build-override]");
        for (field, fa) in &build_override.fields {
            apply_resolved_field(doc, &path, &display, field, fa, findings);
        }
    }
    for (spec, nested) in &requirement.package_overrides {
        let path = vec![
            "profile".to_owned(),
            profile.to_owned(),
            "package".to_owned(),
            spec.clone(),
        ];
        let display = format!("[profile.{profile}.package.{spec}]");
        for (field, fa) in &nested.fields {
            apply_resolved_field(doc, &path, &display, field, fa, findings);
        }
    }
}

fn apply_resolved_field(
    doc: &mut DocumentMut,
    path: &[String],
    display: &str,
    field: &str,
    resolved: &ResolvedProfileScalarField,
    findings: &mut Vec<Finding>,
) {
    let attribution = profile_field_attribution_for(doc, path, field, resolved);
    apply_field(
        doc,
        path,
        display,
        field,
        &resolved.merged,
        &attribution,
        findings,
    );
}

fn profile_field_attribution_for(
    doc: &DocumentMut,
    path: &[String],
    field: &str,
    resolved: &ResolvedProfileScalarField,
) -> Vec<Provenance> {
    let current = field_item(doc, path, field);
    let filtered = resolved
        .collected
        .iter()
        .filter(|(_, assertion)| aqc_toml_engine_core::scalar_assertion_fails(current, assertion))
        .map(|(prov, _)| prov.clone())
        .collect::<Vec<_>>();
    if filtered.is_empty() {
        resolved_attribution(resolved)
    } else {
        filtered
    }
}

/// Apply one scalar assertion to `field` in the table at `path`.
fn apply_field(
    doc: &mut DocumentMut,
    path: &[String],
    display: &str,
    field: &str,
    assertion: &ScalarAssertion<ConfigScalar>,
    attribution: &[Provenance],
    findings: &mut Vec<Finding>,
) {
    match scalar_field_edit(
        format!("{display}.{field}"),
        field_item(doc, path, field),
        assertion,
        attribution,
        findings,
    ) {
        Some(ScalarFieldEdit::Write(item)) => {
            ensure_table_at(doc, path)[field] = item;
        }
        Some(ScalarFieldEdit::Remove) => {
            if let Some(t) = table_at_mut(doc, path) {
                let _ = t.remove(field);
            }
        }
        None => {}
    }
}

/// Read the on-disk item for `field` in the table at `path`, if present.
fn field_item<'a>(doc: &'a DocumentMut, path: &[String], field: &str) -> Option<&'a Item> {
    table_at(doc, path)?.get(field)
}
