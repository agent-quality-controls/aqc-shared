//! Public dependency reconciliation entry points.

#![allow(
    clippy::too_many_arguments,
    clippy::type_complexity,
    reason = "Dependency reconciliation passes resolved requirement sections and output sinks through module boundaries."
)]

use std::collections::{BTreeMap, BTreeSet};

use aqc_file_engine_core::{Finding, ResolvedForbiddenGlobRequirements, ResolvedItemRequirements};
use toml_edit::DocumentMut;

use super::removals::{
    apply_package_glob_forbids, queue_forbidden_matches, queue_membership_extras,
    remove_dependency_entries_once,
};
use super::required::{apply_required, required_file_keys};

use crate::requirement::{DependencyPackageGlob, DependencyRequirement, DependencyScope};
use aqc_toml_engine_core::table_at;

/// Extra generability rule for a dependency-shaped table.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum SetRule {
    /// Standard dependency table.
    Standard,
    /// `[workspace.dependencies]`: `optional` is invalid (cargo rule).
    WorkspaceDeps,
    /// `[patch.<registry>]`: package-only requirements are not writable.
    Patch,
}

/// Apply every scoped dependency-table requirement.
pub(crate) fn apply(
    doc: &mut DocumentMut,
    merged_by_scope: &BTreeMap<DependencyScope, ResolvedItemRequirements<DependencyRequirement>>,
    globs_by_scope: &BTreeMap<
        DependencyScope,
        ResolvedForbiddenGlobRequirements<DependencyPackageGlob>,
    >,
    findings: &mut Vec<Finding>,
) {
    let empty_items = ResolvedItemRequirements::default();
    let empty_globs = ResolvedForbiddenGlobRequirements::default();
    let scopes = merged_by_scope
        .keys()
        .chain(globs_by_scope.keys())
        .collect::<BTreeSet<_>>();
    for scope in scopes {
        let path = scope_path(scope);
        let merged = merged_by_scope.get(scope).unwrap_or(&empty_items);
        let globs = globs_by_scope.get(scope).unwrap_or(&empty_globs);
        apply_set(
            doc,
            &path,
            &scope.table_path(),
            SetRule::Standard,
            merged,
            globs,
            findings,
        );
    }
}

/// The path segments of a `DependencyScope`'s table.
fn scope_path(scope: &DependencyScope) -> Vec<String> {
    let kind = match scope.kind {
        crate::requirement::DependencyKind::Normal => "dependencies",
        crate::requirement::DependencyKind::Dev => "dev-dependencies",
        crate::requirement::DependencyKind::Build => "build-dependencies",
    };
    scope.target.as_ref().map_or_else(
        || vec![kind.to_owned()],
        |t| vec!["target".to_owned(), t.clone(), kind.to_owned()],
    )
}

/// Apply one dependency table to the table at `path`.
pub(crate) fn apply_set(
    doc: &mut DocumentMut,
    path: &[String],
    display_path: &str,
    rule: SetRule,
    merged: &ResolvedItemRequirements<DependencyRequirement>,
    globs: &ResolvedForbiddenGlobRequirements<DependencyPackageGlob>,
    findings: &mut Vec<Finding>,
) {
    let required_file_keys = required_file_keys(merged);
    for entry in merged.required.values() {
        let attribution = entry.attribution();
        let msg = entry
            .collected
            .first()
            .map(|(_, (_, msg))| msg.clone())
            .unwrap_or_default();
        apply_required(
            doc,
            path,
            display_path,
            rule,
            &entry.merged,
            &required_file_keys,
            &msg,
            &attribution,
            findings,
        );
    }

    let mut removals = BTreeMap::new();
    for entry in merged.forbidden.values() {
        let attribution = entry.attribution();
        let msg = entry
            .collected
            .first()
            .map(|(_, msg)| msg.clone())
            .unwrap_or_default();
        queue_forbidden_matches(
            &mut removals,
            table_at(doc, path),
            &entry.merged,
            &msg,
            &attribution,
        );
    }
    apply_package_glob_forbids(
        &mut removals,
        table_at(doc, path),
        display_path,
        globs,
        findings,
    );
    if let Some(membership) = merged.membership() {
        queue_membership_extras(&mut removals, table_at(doc, path), &membership);
    }
    remove_dependency_entries_once(doc, path, display_path, removals, findings);
}
