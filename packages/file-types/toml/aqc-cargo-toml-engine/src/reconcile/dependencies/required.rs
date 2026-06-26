//! Required dependency reconciliation.

#![cfg_attr(
    not(test),
    expect(
        clippy::missing_docs_in_private_items,
        reason = "Private required-dependency helpers are internal reconciliation steps."
    )
)]
#![expect(
    clippy::too_many_arguments,
    clippy::too_many_lines,
    clippy::type_complexity,
    reason = "Required dependency reconciliation keeps Cargo writeability checks with the table update."
)]

use std::collections::{BTreeMap, BTreeSet};

use aqc_file_engine_core::{Finding, Provenance, ResolvedItemRequirements, Severity};
use aqc_toml_engine_core::{ensure_table_at, table_at};
use toml_edit::DocumentMut;

use super::SetRule;
use super::spec_io::{
    find_all_by_package, read_spec, spec_for_write_key, spec_matches, spec_to_item,
};
use crate::requirement::{DependencyRequirement, DependencySpec};

/// Each dependency requirement must be present and partial-match.
pub(super) fn apply_required(
    doc: &mut DocumentMut,
    path: &[String],
    display_path: &str,
    rule: SetRule,
    requirement: &DependencyRequirement,
    required_file_keys: &RequiredFileKeys,
    msg: &str,
    attribution: &[Provenance],
    findings: &mut Vec<Finding>,
) {
    let spec = &requirement.value;
    let name = requirement
        .file_key
        .as_deref()
        .or(requirement.value.package.as_deref())
        .unwrap_or("<unknown>");
    if rule == SetRule::WorkspaceDeps && spec.optional.is_some() {
        findings.push(Finding::InvalidRequirements {
            key: format!("{display_path}.{name}"),
            message: format!("optional is invalid in [workspace.dependencies].{name}. {msg}"),
            contributors: attribution
                .iter()
                .map(|p| (p.policy.clone(), format!("{name} optional")))
                .collect(),
        });
        return;
    }

    let current = table_at(doc, path).and_then(|t| {
        if let Some(file_key) = requirement.file_key.as_deref() {
            read_spec(t, file_key).map(|current_spec| (file_key.to_owned(), current_spec))
        } else {
            let matches = find_all_by_package(t, requirement.value.package.as_deref()?);
            if matches
                .iter()
                .any(|(_, current_spec)| spec_matches(spec, current_spec))
            {
                return Some((String::new(), spec.clone()));
            }
            matches.into_iter().next()
        }
    });
    if current
        .as_ref()
        .is_some_and(|(_, current_spec)| spec_matches(spec, current_spec))
    {
        return;
    }

    let write_key = requirement
        .file_key
        .clone()
        .or_else(|| requirement.value.package.clone());
    let writable = spec.has_source()
        && write_key.is_some()
        && (rule != SetRule::Patch || requirement.file_key.is_some());
    if spec.has_source() && rule == SetRule::Patch && requirement.file_key.is_none() {
        findings.push(Finding::UnwritableRequiredKey {
            key: format!("{display_path}.{name}"),
            expected: format!("{spec:?}"),
            attribution: attribution.to_vec(),
        });
        return;
    }
    if spec.has_source()
        && requirement
            .file_key
            .as_deref()
            .is_some_and(|key| required_file_keys.has_conflicting_packages(key))
    {
        findings.push(Finding::UnwritableRequiredKey {
            key: format!("{display_path}.{name}"),
            expected: format!("{spec:?}"),
            attribution: attribution.to_vec(),
        });
        return;
    }
    if spec.has_source()
        && requirement.file_key.is_none()
        && write_key.as_deref().is_some_and(|key| {
            package_write_key_is_reserved(doc, path, key, spec, required_file_keys)
        })
    {
        findings.push(Finding::UnwritableRequiredKey {
            key: format!("{display_path}.{name}"),
            expected: format!("{spec:?}"),
            attribution: attribution.to_vec(),
        });
        return;
    }

    findings.push(Finding::Mismatch {
        key: format!("{display_path}.{name}"),
        current: current.as_ref().map(|(_, s)| format!("{s:?}")),
        expected: if writable {
            format!("{spec:?}")
        } else {
            format!("{spec:?} (no source: check-only)")
        },
        message: msg.to_owned(),
        severity: Severity::Error,
        attribution: attribution.to_vec(),
    });
    if writable {
        if let Some(write_key) = write_key {
            let write_spec = spec_for_write_key(spec, &write_key);
            ensure_table_at(doc, path)[&write_key] = spec_to_item(&write_spec);
        }
    }
}

#[derive(Debug, Default)]
pub(super) struct RequiredFileKeys {
    packages_by_key: BTreeMap<String, BTreeSet<String>>,
}

impl RequiredFileKeys {
    fn contains(&self, file_key: &str) -> bool {
        self.packages_by_key.contains_key(file_key)
    }

    fn has_conflicting_packages(&self, file_key: &str) -> bool {
        self.packages_by_key
            .get(file_key)
            .is_some_and(|packages| packages.len() > 1)
    }
}

pub(super) fn required_file_keys(
    merged: &ResolvedItemRequirements<DependencyRequirement>,
) -> RequiredFileKeys {
    let mut out = RequiredFileKeys::default();
    for entry in merged.required.values() {
        let Some(file_key) = entry.merged.file_key.as_ref() else {
            continue;
        };
        let effective_package = entry
            .merged
            .value
            .package
            .as_deref()
            .unwrap_or(file_key)
            .to_owned();
        let _ = out
            .packages_by_key
            .entry(file_key.clone())
            .or_default()
            .insert(effective_package);
    }
    out
}

fn package_write_key_is_reserved(
    doc: &DocumentMut,
    path: &[String],
    write_key: &str,
    spec: &DependencySpec,
    required_file_keys: &RequiredFileKeys,
) -> bool {
    if required_file_keys.contains(write_key) {
        return true;
    }
    let Some(package) = spec.package.as_deref() else {
        return false;
    };
    table_at(doc, path)
        .and_then(|table| read_spec(table, write_key))
        .is_some_and(|current| super::spec_io::effective_package(write_key, &current) != package)
}
