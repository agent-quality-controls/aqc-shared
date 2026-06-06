//! Reconcile the `[lints] workspace = <bool>` member opt-in.
//!
//! With `workspace = true` a package inherits every `[workspace.lints.*]`
//! table. The inline-vs-inherit exclusivity rule (cargo rejects `[lints]
//! workspace = true` alongside inline `[lints.<tool>]` tables) is enforced in
//! dispatch before this runs; here we only reconcile the `workspace` key.
//!
//! Lazy: `Present` (check-only) and `Absent` against a missing `[lints]` table
//! create nothing.

use aqc_file_engine_core::{Finding, MergedAssertion, Provenance};
use toml_edit::{DocumentMut, Item, value};

use crate::reconcile::util::{all_provenances, ensure_table, push_mismatch, table_ref};
use crate::requirement::LintsInheritAssertion;

/// Apply the `[lints] workspace` opt-in, if a policy requires it.
pub(crate) fn apply(
    doc: &mut DocumentMut,
    merged: Option<&MergedAssertion<LintsInheritAssertion>>,
    findings: &mut Vec<Finding>,
) {
    let Some(merged) = merged else {
        return;
    };
    let attribution = all_provenances(merged);
    for (_, assertion) in &merged.contributions {
        apply_one(doc, assertion, &attribution, findings);
    }
}

/// Apply a single `LintsInheritAssertion`.
fn apply_one(
    doc: &mut DocumentMut,
    assertion: &LintsInheritAssertion,
    attribution: &[Provenance],
    findings: &mut Vec<Finding>,
) {
    match assertion {
        LintsInheritAssertion::Equals(want, msg) => {
            apply_equals(doc, *want, msg, attribution, findings);
        }
        LintsInheritAssertion::Present(msg) => apply_present(doc, msg, attribution, findings),
        LintsInheritAssertion::Absent(msg) => apply_absent(doc, msg, attribution, findings),
    }
}

/// Read the on-disk `[lints].workspace` bool, if present.
fn current_workspace(doc: &DocumentMut) -> Option<bool> {
    table_ref(doc, "lints").and_then(|t| t.get("workspace").and_then(Item::as_bool))
}

/// `[lints] workspace == want`.
fn apply_equals(
    doc: &mut DocumentMut,
    want: bool,
    msg: &str,
    attribution: &[Provenance],
    findings: &mut Vec<Finding>,
) {
    if current_workspace(doc) == Some(want) {
        return;
    }
    push_mismatch(
        findings,
        "[lints].workspace".to_owned(),
        current_workspace(doc).map(|b| b.to_string()),
        want.to_string(),
        msg.to_owned(),
        attribution,
    );
    ensure_table(doc, "lints")["workspace"] = value(want);
}

/// The `workspace` key is set (check-only).
fn apply_present(
    doc: &DocumentMut,
    msg: &str,
    attribution: &[Provenance],
    findings: &mut Vec<Finding>,
) {
    if table_ref(doc, "lints").is_some_and(|t| t.contains_key("workspace")) {
        return;
    }
    push_mismatch(
        findings,
        "[lints].workspace".to_owned(),
        None,
        "any value (Present)".to_owned(),
        msg.to_owned(),
        attribution,
    );
}

/// The `workspace` key is not set (vacuous when already absent).
fn apply_absent(
    doc: &mut DocumentMut,
    msg: &str,
    attribution: &[Provenance],
    findings: &mut Vec<Finding>,
) {
    if !table_ref(doc, "lints").is_some_and(|t| t.contains_key("workspace")) {
        return;
    }
    push_mismatch(
        findings,
        "[lints].workspace".to_owned(),
        current_workspace(doc).map(|b| b.to_string()),
        "absent".to_owned(),
        msg.to_owned(),
        attribution,
    );
    if let Some(t) = doc.get_mut("lints").and_then(Item::as_table_mut) {
        let _ = t.remove("workspace");
    }
}
