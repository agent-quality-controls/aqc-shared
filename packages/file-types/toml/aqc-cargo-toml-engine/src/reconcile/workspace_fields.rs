//! Reconcile direct `[workspace].<key>` assertions (resolver, members, ...).
//!
//! Lazy: check-only assertions and vacuous removals never create `[workspace]`.

#![cfg_attr(
    not(test),
    expect(
        clippy::missing_docs_in_private_items,
        reason = "Private workspace-field helpers are internal reconciliation steps."
    )
)]
#![expect(
    clippy::type_complexity,
    reason = "Workspace-field reconciliation consumes resolved requirement map shapes."
)]

use std::collections::BTreeMap;

use aqc_file_engine_core as core_types;
use aqc_toml_engine_core::{ScalarFieldEdit, scalar_field_edit};
use toml_edit as toml;

use crate::requirement as req;

/// The finding-path prefix for direct workspace keys.
const PREFIX: &str = "workspace";

/// Apply every direct `[workspace].<key>` requirement.
pub(crate) fn apply(
    doc: &mut toml::DocumentMut,
    merged_by_key: &BTreeMap<
        String,
        core_types::ResolvedRequirement<
            req::ResolvedWorkspaceFieldAssertion,
            req::WorkspaceFieldAssertion,
        >,
    >,
    findings: &mut Vec<core_types::Finding>,
) {
    for (key, merged) in merged_by_key {
        let attribution = attribution_for(doc, key, merged);
        apply_one(doc, key, &merged.merged, &attribution, findings);
    }
}

/// Read the on-disk item for `key` under `[workspace]`, if present.
fn current_item<'a>(doc: &'a toml::DocumentMut, key: &str) -> Option<&'a toml::Item> {
    aqc_toml_engine_core::table_ref(doc, PREFIX).and_then(|t| t.get(key))
}

/// The mutable `[workspace]` table if it already exists (removals only).
fn workspace_mut_existing(doc: &mut toml::DocumentMut) -> Option<&mut toml::Table> {
    doc.get_mut(PREFIX).and_then(toml::Item::as_table_mut)
}

fn attribution_for(
    doc: &toml::DocumentMut,
    key: &str,
    resolved: &core_types::ResolvedRequirement<
        req::ResolvedWorkspaceFieldAssertion,
        req::WorkspaceFieldAssertion,
    >,
) -> Vec<core_types::Provenance> {
    let current = current_item(doc, key);
    let filtered = resolved
        .collected
        .iter()
        .filter(|(_, assertion)| assertion_fails(current, assertion))
        .map(|(prov, _)| prov.clone())
        .collect::<Vec<_>>();
    if filtered.is_empty() {
        aqc_toml_engine_core::attribution(resolved)
    } else {
        filtered
    }
}

fn assertion_fails(current: Option<&toml::Item>, assertion: &req::WorkspaceFieldAssertion) -> bool {
    match assertion {
        req::WorkspaceFieldAssertion::Scalar(assertion) => match assertion {
            core_types::ScalarAssertion::AtLeast(..)
            | core_types::ScalarAssertion::AtMost(..)
            | core_types::ScalarAssertion::Range(..) => true,
            core_types::ScalarAssertion::Equals(..)
            | core_types::ScalarAssertion::OneOf(..)
            | core_types::ScalarAssertion::Present(_)
            | core_types::ScalarAssertion::Absent(_) => {
                aqc_toml_engine_core::scalar_assertion_fails(current, assertion)
            }
        },
        req::WorkspaceFieldAssertion::List(_) => false,
    }
}

/// Apply a single resolved workspace field assertion.
fn apply_one(
    doc: &mut toml::DocumentMut,
    key: &str,
    assertion: &req::ResolvedWorkspaceFieldAssertion,
    attribution: &[core_types::Provenance],
    findings: &mut Vec<core_types::Finding>,
) {
    match assertion {
        req::ResolvedWorkspaceFieldAssertion::Scalar(assertion) => {
            apply_scalar(doc, key, assertion, attribution, findings);
        }
        req::ResolvedWorkspaceFieldAssertion::List(list) => {
            let current = aqc_toml_engine_core::table_ref(doc, PREFIX).map_or_else(Vec::new, |t| {
                aqc_toml_engine_core::table_list_values(t, key)
            });
            if let Some(updated) = aqc_toml_engine_core::reconcile_table_list_field(
                format!("[{PREFIX}].{key}"),
                current,
                list,
                findings,
            ) {
                aqc_toml_engine_core::write_table_list(
                    aqc_toml_engine_core::ensure_table(doc, PREFIX),
                    key,
                    &updated,
                );
            }
        }
    }
}

/// `key == want`.
fn apply_scalar(
    doc: &mut toml::DocumentMut,
    key: &str,
    assertion: &core_types::ScalarAssertion<core_types::ConfigScalar>,
    attribution: &[core_types::Provenance],
    findings: &mut Vec<core_types::Finding>,
) {
    match scalar_field_edit(
        format!("[{PREFIX}].{key}"),
        current_item(doc, key),
        assertion,
        attribution,
        findings,
    ) {
        Some(ScalarFieldEdit::Write(item)) => {
            aqc_toml_engine_core::ensure_table(doc, PREFIX)[key] = item;
        }
        Some(ScalarFieldEdit::Remove) => {
            if let Some(t) = workspace_mut_existing(doc) {
                let _ = t.remove(key);
            }
        }
        None => {}
    }
}
