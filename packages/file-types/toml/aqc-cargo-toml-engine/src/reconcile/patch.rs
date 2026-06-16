//! Reconcile `[patch.<registry>]` tables. Reuses the dependency helpers
//! against the nested `[patch.<registry>]` table.

use std::collections::BTreeMap;

use aqc_file_engine_core::{Finding, ResolvedItemRequirements};
use toml_edit::DocumentMut;

use crate::reconcile::dependencies::{SetRule, apply_set};
use crate::requirement::DependencyRequirement;

/// Apply every `[patch.<registry>]` requirement.
pub(crate) fn apply(
    doc: &mut DocumentMut,
    merged_by_registry: &BTreeMap<String, ResolvedItemRequirements<DependencyRequirement>>,
    findings: &mut Vec<Finding>,
) {
    for (registry, merged) in merged_by_registry {
        let path = vec!["patch".to_owned(), registry.clone()];
        let display = format!("[patch.{registry}]");
        // Package-only patch entries are not writable; the shared helper emits
        // UnwritableRequiredKey when it cannot use a `file_key`.
        apply_set(doc, &path, &display, SetRule::Patch, merged, findings);
    }
}

/// Apply the single `[workspace.dependencies]` requirement.
pub(crate) fn apply_workspace_dependencies(
    doc: &mut DocumentMut,
    merged: Option<&ResolvedItemRequirements<DependencyRequirement>>,
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
