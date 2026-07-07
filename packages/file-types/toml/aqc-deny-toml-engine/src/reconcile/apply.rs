//! Apply resolved deny.toml requirements to a TOML document.

use aqc_file_engine_core::Finding;
use aqc_toml_engine_core::push_mismatch;
use toml_edit::{DocumentMut, Item};

use crate::requirement::ResolvedDenyTomlRequirements;

use super::{
    closed::apply_closed_settings,
    items::apply_items,
    lists::apply_lists,
    scalars::{apply_scalars, touch_core_scalar_helpers},
};

pub(crate) fn apply(
    doc: &mut DocumentMut,
    requirement: &ResolvedDenyTomlRequirements,
    findings: &mut Vec<Finding>,
) {
    touch_core_scalar_helpers(findings);
    reject_unsupported_source_key(doc, findings);

    apply_items(doc, requirement, findings);
    apply_scalars(doc, requirement, findings);
    apply_lists(doc, requirement, findings);
    apply_closed_settings(doc, requirement, findings);
}

fn reject_unsupported_source_key(doc: &mut DocumentMut, findings: &mut Vec<Finding>) {
    let Some(sources) = doc.get_mut("sources").and_then(Item::as_table_mut) else {
        return;
    };
    if sources.remove("unused-allowed-org").is_some() {
        push_mismatch(
            findings,
            "sources.unused-allowed-org".to_owned(),
            Some("present".to_owned()),
            "absent".to_owned(),
            "unsupported by cargo-deny 0.19.4".to_owned(),
            &[],
        );
    }
}
