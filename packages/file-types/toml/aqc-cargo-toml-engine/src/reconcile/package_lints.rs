//! Reconcile the `[lints]` either/or key: inherit the workspace tables
//! (`workspace = <bool>`) or carry inline `[lints.<tool>]` tables.
//!
//! Lazy: nothing is created unless a write is needed.

use aqc_file_engine_core::{Finding, Provenance};
use toml_edit::{DocumentMut, Item, value};

use crate::reconcile::lints::{self, LintRoot};
use crate::reconcile::util::{ensure_table, push_mismatch, table_ref};
use crate::requirement::ResolvedPackageLintsAssertion;

/// Apply the `[lints]` decision, if any policy requires one.
pub(crate) fn apply(
    doc: &mut DocumentMut,
    pairs: Option<&ResolvedPackageLintsAssertion>,
    findings: &mut Vec<Finding>,
) {
    let Some(pairs) = pairs else {
        return;
    };
    match pairs {
        ResolvedPackageLintsAssertion::Inherit(resolved) => {
            let attribution = resolved
                .collected
                .iter()
                .map(|(prov, _)| prov.clone())
                .collect::<Vec<_>>();
            let msg = resolved
                .collected
                .first()
                .map(|(_, (_, msg))| msg.clone())
                .unwrap_or_default();
            apply_inherit(doc, resolved.merged, &msg, &attribution, findings);
        }
        ResolvedPackageLintsAssertion::Inline(tools) => {
            lints::apply(doc, LintRoot::Package, tools, findings);
        }
    }
}

/// Read the on-disk `[lints].workspace` bool, if present.
fn current_workspace(doc: &DocumentMut) -> Option<bool> {
    table_ref(doc, "lints").and_then(|t| t.get("workspace").and_then(Item::as_bool))
}

/// `[lints] workspace == want`.
fn apply_inherit(
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
