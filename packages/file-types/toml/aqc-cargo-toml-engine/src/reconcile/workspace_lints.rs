//! Reconcile `[workspace.lints.<tool>]` tables. Same per-tool logic as
//! `[lints.<tool>]`, but at a deeper path.

use std::collections::BTreeMap;

use aqc_file_engine_core::{Finding, MergedAssertion};

use crate::reconcile::lints;
use crate::reconcile::util::{get_or_create_nested_table_mut, get_or_create_table_mut};
use crate::requirement::LintLevelsAssertion;

/// Apply every `[workspace.lints.<tool>]` contribution.
#[expect(
    clippy::type_complexity,
    reason = "BTreeMap<String, MergedAssertion<...>> is the natural section input shape"
)]
pub(crate) fn apply(
    doc: &mut toml_edit::DocumentMut,
    merged_by_tool: &BTreeMap<String, MergedAssertion<LintLevelsAssertion>>,
    findings: &mut Vec<Finding>,
) {
    if merged_by_tool.is_empty() {
        return;
    }
    let workspace_root = get_or_create_table_mut(doc, "workspace");
    let lints_root = get_or_create_nested_table_mut(workspace_root, "lints");
    for (tool, merged) in merged_by_tool {
        lints::apply_tool(lints_root, "workspace.lints", tool, merged, findings);
    }
}
