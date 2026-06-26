//! List setting reconciliation.

use aqc_file_engine_core::{Finding, ResolvedListRequirements};
use aqc_toml_engine_core::{ListFieldKeyStyle, reconcile_list_field, report_list_shape};
use toml_edit::DocumentMut;

use aqc_toml_engine_core::{list_values, write_list};

/// Applies contains, excludes, and exact list requirements.
pub(super) fn apply_list(
    doc: &mut DocumentMut,
    key: &str,
    requirements: &ResolvedListRequirements,
    findings: &mut Vec<Finding>,
) {
    if report_list_shape(doc, key, requirements, findings) {
        let values = list_values(doc, key);
        write_list(doc, key, &values);
    }
    let values = list_values(doc, key);
    if let Some(updated) = reconcile_list_field(
        key.to_owned(),
        values,
        requirements,
        ListFieldKeyStyle::FieldItem,
        findings,
    ) {
        write_list(doc, key, &updated);
    }
}
