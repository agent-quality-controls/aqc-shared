//! Reconcile the `[lints]` either/or key: inherit the workspace tables
//! (`workspace = <bool>`) or carry inline `[lints.<tool>]` tables.
//!
//! Lazy: nothing is created unless a write is needed.

use aqc_file_engine_core::{ConfigScalar, Finding, Provenance, ScalarAssertion};
use aqc_toml_engine_core::{ScalarFieldEdit, ensure_table, scalar_field_edit, table_ref};
use toml_edit::DocumentMut;

use crate::reconcile::lints::{self, LintRoot};
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
            let attribution = resolved.attribution();
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

/// `[lints] workspace == want`.
fn apply_inherit(
    doc: &mut DocumentMut,
    want: bool,
    msg: &str,
    attribution: &[Provenance],
    findings: &mut Vec<Finding>,
) {
    let assertion = ScalarAssertion::Equals(ConfigScalar::Bool(want), msg.to_owned());
    let current = table_ref(doc, "lints").and_then(|t| t.get("workspace"));
    if let Some(ScalarFieldEdit::Write(item)) = scalar_field_edit(
        "[lints].workspace".to_owned(),
        current,
        &assertion,
        attribution,
        findings,
    ) {
        ensure_table(doc, "lints")["workspace"] = item;
    }
}
