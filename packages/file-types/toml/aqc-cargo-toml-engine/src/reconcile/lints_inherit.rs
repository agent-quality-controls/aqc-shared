//! Reconcile the `[lints] workspace = <bool>` opt-in.
//!
//! This is the multi-tool inherit switch: with `workspace = true` a package
//! inherits every `[workspace.lints.{rust,clippy,rustdoc}]` table. Without it
//! the workspace lint config is inert (lints never apply to the crate).

use aqc_file_engine_core::{Finding, MergedAssertion, Severity};
use toml_edit::{DocumentMut, Item, value};

use crate::reconcile::util::{all_provenances, get_or_create_table_mut};
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
    // After merge every contribution agrees; read the resolved value.
    let Some((_, LintsInheritAssertion::Workspace(want))) = merged.contributions.first() else {
        return;
    };
    let want = *want;
    let attribution = all_provenances(merged);
    let lints = get_or_create_table_mut(doc, "lints");
    let current = lints.get("workspace").and_then(Item::as_bool);
    if current == Some(want) {
        return;
    }
    findings.push(Finding::Mismatch {
        path: "[lints].workspace".to_owned(),
        current: current.map(|b| b.to_string()),
        expected: want.to_string(),
        message: String::new(),
        severity: Severity::Error,
        attribution,
    });
    lints["workspace"] = value(want);
}
