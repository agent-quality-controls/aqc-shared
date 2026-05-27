//! Reconcile `[workspace.package].<field>`.

use std::collections::BTreeMap;

use aqc_file_engine_core::{Finding, MergedAssertion};

use crate::reconcile::package_fields;
use crate::reconcile::util::{get_or_create_nested_table_mut, get_or_create_table_mut};
use crate::requirement::PackageFieldAssertion;

/// Apply every `[workspace.package].<field>` contribution.
#[expect(
    clippy::type_complexity,
    reason = "BTreeMap<String, MergedAssertion<...>> is the natural section input shape"
)]
pub(crate) fn apply(
    doc: &mut toml_edit::DocumentMut,
    merged_by_field: &BTreeMap<String, MergedAssertion<PackageFieldAssertion>>,
    findings: &mut Vec<Finding>,
) {
    if merged_by_field.is_empty() {
        return;
    }
    let workspace = get_or_create_table_mut(doc, "workspace");
    let package = get_or_create_nested_table_mut(workspace, "package");
    for (field, merged) in merged_by_field {
        package_fields::apply_into_table(package, "workspace.package", field, merged, findings);
    }
}
