//! Reconcile the `[lints]` either/or key: inherit the workspace tables
//! (`workspace = <bool>`) or carry inline `[lints.<tool>]` tables.
//!
//! Post-merge the collected list carries one resolved assertion re-paired
//! with every contributing provenance; mixed inherit/inline never reaches
//! here (the merge surfaces it as `ConflictingRequirements`).
//!
//! Lazy: nothing is created unless a write is needed.

#![expect(
    clippy::type_complexity,
    reason = "Collected assertions are plainly Vec<(Provenance, A)> and per-key maps of them; the shapes are declared openly at every signature instead of hidden behind wrapper types or aliases (taxonomy decision 2026-06-07)."
)]
use std::collections::BTreeMap;

use aqc_file_engine_core::{Finding, Provenance};
use toml_edit::{DocumentMut, Item, value};

use crate::reconcile::lints::{self, LintRoot};
use crate::reconcile::util::{all_provenances, ensure_table, push_mismatch, table_ref};
use crate::requirement::PackageLintsAssertion;

/// Apply the `[lints]` decision, if any policy requires one.
pub(crate) fn apply(
    doc: &mut DocumentMut,
    pairs: Option<&Vec<(Provenance, PackageLintsAssertion)>>,
    findings: &mut Vec<Finding>,
) {
    let Some(pairs) = pairs else {
        return;
    };
    let Some((_, assertion)) = pairs.first() else {
        return;
    };
    let attribution = all_provenances(pairs);
    match assertion {
        PackageLintsAssertion::Inherit(want, msg) => {
            apply_inherit(doc, *want, msg, &attribution, findings);
        }
        PackageLintsAssertion::Inline(tools) => {
            // Re-pair each tool table with the full attribution and reuse the
            // lint-table reconcile.
            let by_tool: BTreeMap<String, Vec<(Provenance, _)>> = tools
                .iter()
                .map(|(tool, table)| {
                    let attributed = attribution
                        .iter()
                        .map(|p| (p.clone(), table.clone()))
                        .collect();
                    (tool.clone(), attributed)
                })
                .collect();
            lints::apply(doc, LintRoot::Package, &by_tool, findings);
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
