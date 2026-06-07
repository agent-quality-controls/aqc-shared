//! Reconcile `[patch.<registry>]` tables. Reuses the dependency-set helpers
//! against the nested `[patch.<registry>]` table.

#![expect(
    clippy::type_complexity,
    reason = "Collected assertions are plainly Vec<(Provenance, A)> and per-key maps of them; the shapes are declared openly at every signature instead of hidden behind wrapper types or aliases (taxonomy decision 2026-06-07)."
)]
use std::collections::BTreeMap;

use aqc_file_engine_core::{Finding, Provenance};
use toml_edit::DocumentMut;

use crate::reconcile::dependencies::{SetRule, apply_set};
use crate::requirement::DependencySetAssertion;

/// Apply every `[patch.<registry>]` contribution.
pub(crate) fn apply(
    doc: &mut DocumentMut,
    merged_by_registry: &BTreeMap<String, Vec<(Provenance, DependencySetAssertion)>>,
    findings: &mut Vec<Finding>,
) {
    for (registry, merged) in merged_by_registry {
        let path = vec!["patch".to_owned(), registry.clone()];
        let display = format!("[patch.{registry}]");
        apply_set(doc, &path, &display, SetRule::Standard, merged, findings);
    }
}

/// Apply the single `[workspace.dependencies]` contribution.
pub(crate) fn apply_workspace_dependencies(
    doc: &mut DocumentMut,
    merged: Option<&Vec<(Provenance, DependencySetAssertion)>>,
    findings: &mut Vec<Finding>,
) {
    let Some(merged) = merged else { return };
    let path = vec!["workspace".to_owned(), "dependencies".to_owned()];
    apply_set(
        doc,
        &path,
        "[workspace.dependencies]",
        SetRule::WorkspaceDeps,
        merged,
        findings,
    );
}
