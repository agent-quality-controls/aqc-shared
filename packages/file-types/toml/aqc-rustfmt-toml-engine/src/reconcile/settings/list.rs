//! List setting reconciliation.

use aqc_file_engine_core::{Finding, ResolvedListRequirements, apply_list_requirements};
use aqc_toml_engine_core::{ListFieldKeyStyle, reconcile_optional_list_field, report_list_shape};
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
        let updated = apply_list_requirements(&list_values(doc, key), requirements);
        write_list(doc, key, &updated);
        return;
    }
    let values = doc.get(key).map(|_| list_values(doc, key));
    if let Some(updated) = reconcile_optional_list_field(
        key.to_owned(),
        values,
        requirements,
        ListFieldKeyStyle::FieldItem,
        findings,
    ) {
        write_list(doc, key, &updated);
    }
}
