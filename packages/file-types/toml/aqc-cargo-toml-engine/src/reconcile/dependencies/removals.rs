//! Dependency removal planning and execution.

#![cfg_attr(
    not(test),
    expect(
        clippy::missing_docs_in_private_items,
        reason = "Private dependency removal helpers are internal reconciliation steps."
    )
)]
#![allow(
    clippy::type_complexity,
    reason = "Dependency match helpers return file-key/spec pairs from Cargo tables."
)]

use std::collections::{BTreeMap, BTreeSet};

use aqc_file_engine_core::{Finding, Provenance, ResolvedForbiddenGlobRequirements, Severity};
use globset::{GlobBuilder, GlobMatcher};
use toml_edit::{DocumentMut, TableLike};

use super::spec_io::{effective_package, find_all_by_package, read_spec};
use crate::requirement::{
    DependencyForbiddenGlobConflictBlocks, DependencyPackageGlob, DependencyRequirement,
    DependencySpec,
};
use aqc_toml_engine_core::table_at_mut;

#[derive(Debug)]
pub(super) struct PlannedDependencyRemoval {
    current: DependencySpec,
    expected: BTreeSet<String>,
    messages: BTreeSet<String>,
    attribution: BTreeSet<Provenance>,
}

fn queue_removal(
    removals: &mut BTreeMap<String, PlannedDependencyRemoval>,
    file_key: String,
    current: DependencySpec,
    expected: &str,
    msg: &str,
    attribution: &[Provenance],
) {
    let entry = removals
        .entry(file_key)
        .or_insert_with(|| PlannedDependencyRemoval {
            current,
            expected: BTreeSet::new(),
            messages: BTreeSet::new(),
            attribution: BTreeSet::new(),
        });
    let _ = entry.expected.insert(expected.to_owned());
    if !msg.is_empty() {
        let _ = entry.messages.insert(msg.to_owned());
    }
    entry.attribution.extend(attribution.iter().cloned());
}

/// Each named entry must be absent (vacuous when the table is missing).
pub(super) fn queue_forbidden_matches(
    removals: &mut BTreeMap<String, PlannedDependencyRemoval>,
    table: Option<&dyn TableLike>,
    requirement: &DependencyRequirement,
    msg: &str,
    attribution: &[Provenance],
) {
    let matches = table
        .map(|table| read_forbidden_matches(table, requirement))
        .unwrap_or_default();
    for (name, current) in matches {
        queue_removal(removals, name, current, "absent", msg, attribution);
    }
}

fn read_forbidden_matches(
    table: &dyn TableLike,
    requirement: &DependencyRequirement,
) -> Vec<(String, DependencySpec)> {
    if let Some(file_key) = requirement.file_key.as_deref() {
        return read_spec(table, file_key)
            .map(|spec| vec![(file_key.to_owned(), spec)])
            .unwrap_or_default();
    }
    let Some(package) = requirement.value.package.as_deref() else {
        return Vec::new();
    };
    find_all_by_package(table, package)
}

pub(super) fn apply_package_glob_forbids(
    removals: &mut BTreeMap<String, PlannedDependencyRemoval>,
    table: Option<&dyn TableLike>,
    display_path: &str,
    globs: &ResolvedForbiddenGlobRequirements<DependencyPackageGlob>,
    glob_conflicts: &DependencyForbiddenGlobConflictBlocks,
    findings: &mut Vec<Finding>,
) {
    for (glob_identity, entry) in &globs.globs {
        if glob_conflicts.package_globs.contains(glob_identity) {
            continue;
        }
        let glob = &entry.merged;
        let attribution = entry
            .collected
            .iter()
            .map(|(prov, _)| prov.clone())
            .collect::<Vec<_>>();
        let message = entry
            .collected
            .first()
            .map(|(_, msg)| msg.clone())
            .unwrap_or_default();
        let matcher = match compile_package_glob(glob) {
            Ok(matcher) => matcher,
            Err(error_message) => {
                findings.push(Finding::InvalidRequirements {
                    key: format!("{display_path}.{}", glob.glob),
                    message: error_message,
                    contributors: entry
                        .collected
                        .iter()
                        .map(|(prov, contributor_message)| {
                            (prov.policy.clone(), contributor_message.clone())
                        })
                        .collect(),
                });
                continue;
            }
        };
        let matched_dependencies = table
            .map(|table| read_package_glob_matches(table, &matcher))
            .unwrap_or_default();
        for (file_key, current) in matched_dependencies {
            queue_removal(
                removals,
                file_key,
                current,
                "absent (package glob)",
                &message,
                &attribution,
            );
        }
    }
}

fn compile_package_glob(glob: &DependencyPackageGlob) -> Result<GlobMatcher, String> {
    GlobBuilder::new(&glob.glob)
        .literal_separator(true)
        .build()
        .map(|glob| glob.compile_matcher())
        .map_err(|err| format!("invalid dependency package glob {}: {err}", glob.glob))
}

fn read_package_glob_matches(
    table: &dyn TableLike,
    matcher: &GlobMatcher,
) -> Vec<(String, DependencySpec)> {
    table
        .iter()
        .filter_map(|(file_key, _)| {
            let spec = read_spec(table, file_key)?;
            let package = effective_package(file_key, &spec);
            matcher
                .is_match(package)
                .then(|| (file_key.to_owned(), spec))
        })
        .collect()
}

/// Drop on-disk entries not allowed by the closed collection.
pub(super) fn queue_exact_extras(
    removals: &mut BTreeMap<String, PlannedDependencyRemoval>,
    table: Option<&dyn TableLike>,
    allowed: &[DependencyRequirement],
    attribution: &[Provenance],
) {
    let Some(table) = table else {
        return;
    };
    let extras = table
        .iter()
        .filter_map(|(file_key, _)| {
            let spec = read_spec(table, file_key)?;
            let effective_package = effective_package(file_key, &spec).to_owned();
            let allowed = allowed.iter().any(|requirement| {
                requirement_matches_file_item(requirement, file_key, &effective_package)
            });
            (!allowed).then(|| (file_key.to_owned(), spec))
        })
        .collect::<Vec<_>>();
    for (extra, current) in &extras {
        queue_removal(
            removals,
            extra.clone(),
            current.clone(),
            "absent (closed collection)",
            "",
            attribution,
        );
    }
}

pub(super) fn remove_dependency_entries_once(
    doc: &mut DocumentMut,
    path: &[String],
    display_path: &str,
    removals: BTreeMap<String, PlannedDependencyRemoval>,
    findings: &mut Vec<Finding>,
) {
    for (file_key, removal) in removals {
        findings.push(Finding::Mismatch {
            key: format!("{display_path}.{file_key}"),
            current: Some(format!("{:?}", removal.current)),
            expected: removal.expected.into_iter().collect::<Vec<_>>().join("; "),
            message: removal.messages.into_iter().collect::<Vec<_>>().join("; "),
            severity: Severity::Error,
            attribution: removal.attribution.into_iter().collect(),
        });
        if let Some(t) = table_at_mut(doc, path) {
            let _ = t.remove(file_key.as_str());
        }
    }
}

fn requirement_matches_file_item(
    requirement: &DependencyRequirement,
    file_key: &str,
    effective_package: &str,
) -> bool {
    requirement.file_key.as_deref().map_or_else(
        || requirement.value.package.as_deref() == Some(effective_package),
        |required_key| required_key == file_key,
    )
}
