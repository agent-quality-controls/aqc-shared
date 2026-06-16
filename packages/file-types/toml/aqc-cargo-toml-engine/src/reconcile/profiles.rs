//! Reconcile `[profile.<name>]` tables, including
//! `[profile.<name>.package.<spec>]` overrides and `[profile.<name>.build-override]`.
//!
//! Lazy: check-only field assertions (`OneOf`, `Present`) and vacuous removals
//! create no tables.

use std::collections::BTreeMap;

use aqc_file_engine_core::{ConfigScalar, Finding, Provenance, ResolvedRequirement};
use toml_edit::{DocumentMut, Item};

use crate::reconcile::util::{
    attribution as resolved_attribution, ensure_table_at, push_mismatch, render_item,
    render_scalar, scalar_item, scalar_matches, table_at, table_at_mut,
};
use crate::requirement::{ProfileFieldAssertion, ResolvedProfileRequirements};

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
    resolved: &ResolvedRequirement<ProfileFieldAssertion, ProfileFieldAssertion>,
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
    resolved: &ResolvedRequirement<ProfileFieldAssertion, ProfileFieldAssertion>,
) -> Vec<Provenance> {
    let current = field_item(doc, path, field);
    let filtered = resolved
        .collected
        .iter()
        .filter(|(_, assertion)| profile_assertion_fails(current, assertion))
        .map(|(prov, _)| prov.clone())
        .collect::<Vec<_>>();
    if filtered.is_empty() {
        resolved_attribution(resolved)
    } else {
        filtered
    }
}

fn profile_assertion_fails(current: Option<&Item>, assertion: &ProfileFieldAssertion) -> bool {
    match assertion {
        ProfileFieldAssertion::Equals(want, _) => {
            !current.is_some_and(|item| scalar_matches(item, want))
        }
        ProfileFieldAssertion::OneOf(allowed, _) => {
            !current.is_some_and(|item| allowed.iter().any(|allowed| scalar_matches(item, allowed)))
        }
        ProfileFieldAssertion::Present(_) => current.is_none(),
        ProfileFieldAssertion::Absent(_) => current.is_some(),
    }
}

/// Apply one `ProfileFieldAssertion` to `field` in the table at `path`.
fn apply_field(
    doc: &mut DocumentMut,
    path: &[String],
    display: &str,
    field: &str,
    assertion: &ProfileFieldAssertion,
    attribution: &[Provenance],
    findings: &mut Vec<Finding>,
) {
    match assertion {
        ProfileFieldAssertion::Equals(want, msg) => {
            apply_equals(doc, path, display, field, want, msg, attribution, findings);
        }
        ProfileFieldAssertion::OneOf(allowed, msg) => {
            apply_one_of(
                doc,
                path,
                display,
                field,
                allowed,
                msg,
                attribution,
                findings,
            );
        }
        ProfileFieldAssertion::Present(msg) => {
            apply_present(doc, path, display, field, msg, attribution, findings);
        }
        ProfileFieldAssertion::Absent(msg) => {
            apply_absent(doc, path, display, field, msg, attribution, findings);
        }
    }
}

/// `field == want`.
#[expect(
    clippy::too_many_arguments,
    reason = "the path-addressed field appliers carry doc, path, display, field, value, msg, attribution, findings; each is a distinct input with no natural grouping."
)]
fn apply_equals(
    doc: &mut DocumentMut,
    path: &[String],
    display: &str,
    field: &str,
    want: &ConfigScalar,
    msg: &str,
    attribution: &[Provenance],
    findings: &mut Vec<Finding>,
) {
    let current = field_item(doc, path, field);
    if current.is_some_and(|it| scalar_matches(it, want)) {
        return;
    }
    let rendered = field_item(doc, path, field).and_then(render_item);
    push_mismatch(
        findings,
        format!("{display}.{field}"),
        rendered,
        render_scalar(want),
        msg.to_owned(),
        attribution,
    );
    ensure_table_at(doc, path)[field] = scalar_item(want);
}

/// `field ∈ allowed` (check-only).
#[expect(
    clippy::too_many_arguments,
    reason = "see apply_equals: distinct path-addressed inputs, no natural grouping."
)]
fn apply_one_of(
    doc: &DocumentMut,
    path: &[String],
    display: &str,
    field: &str,
    allowed: &[ConfigScalar],
    msg: &str,
    attribution: &[Provenance],
    findings: &mut Vec<Finding>,
) {
    let current = field_item(doc, path, field);
    if current.is_some_and(|it| allowed.iter().any(|a| scalar_matches(it, a))) {
        return;
    }
    let rendered = current.and_then(render_item);
    let allowed_render: Vec<String> = allowed.iter().map(render_scalar).collect();
    push_mismatch(
        findings,
        format!("{display}.{field}"),
        rendered,
        format!("one of {allowed_render:?}"),
        msg.to_owned(),
        attribution,
    );
}

/// `field` must be set (check-only).
fn apply_present(
    doc: &DocumentMut,
    path: &[String],
    display: &str,
    field: &str,
    msg: &str,
    attribution: &[Provenance],
    findings: &mut Vec<Finding>,
) {
    if field_item(doc, path, field).is_some() {
        return;
    }
    push_mismatch(
        findings,
        format!("{display}.{field}"),
        None,
        "any value (Present)".to_owned(),
        msg.to_owned(),
        attribution,
    );
}

/// `field` must not be set (vacuous when already absent).
fn apply_absent(
    doc: &mut DocumentMut,
    path: &[String],
    display: &str,
    field: &str,
    msg: &str,
    attribution: &[Provenance],
    findings: &mut Vec<Finding>,
) {
    let rendered = field_item(doc, path, field).and_then(render_item);
    if field_item(doc, path, field).is_none() {
        return;
    }
    push_mismatch(
        findings,
        format!("{display}.{field}"),
        rendered,
        "absent".to_owned(),
        msg.to_owned(),
        attribution,
    );
    if let Some(t) = table_at_mut(doc, path) {
        let _ = t.remove(field);
    }
}

/// Read the on-disk item for `field` in the table at `path`, if present.
fn field_item<'a>(doc: &'a DocumentMut, path: &[String], field: &str) -> Option<&'a Item> {
    table_at(doc, path)?.get(field)
}
