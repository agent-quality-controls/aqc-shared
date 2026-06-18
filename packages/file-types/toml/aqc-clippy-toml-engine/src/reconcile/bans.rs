//! Reconciliation for clippy.toml ban tables.

#![cfg_attr(
    not(test),
    expect(
        clippy::missing_docs_in_private_items,
        reason = "Private ban-table helpers are internal reconciliation steps."
    )
)]

use std::collections::BTreeSet;

use aqc_file_engine_core::{
    Finding, Provenance, ResolvedForbiddenGlobRequirements, ResolvedItemRequirements, Severity,
};
use globset::{GlobBuilder, GlobMatcher};
use toml_edit::{Array, DocumentMut, InlineTable, Item, Value};

use crate::requirement::{BanEntry, ClippyForbiddenGlobConflictBlocks, ClippyPathGlob};

pub(crate) fn apply(
    doc: &mut DocumentMut,
    table_key: &str,
    merged: &ResolvedItemRequirements<BanEntry>,
    globs: &ResolvedForbiddenGlobRequirements<ClippyPathGlob>,
    glob_conflicts: &ClippyForbiddenGlobConflictBlocks,
    findings: &mut Vec<Finding>,
) {
    if merged.required.is_empty()
        && merged.banned.is_empty()
        && merged.closed_by.is_empty()
        && globs.globs.is_empty()
    {
        return;
    }

    let array = if merged.required.is_empty() {
        let Some(item) = doc.get_mut(table_key) else {
            return;
        };
        if !item.is_array() {
            push_malformed_array_finding(table_key, item, merged, findings);
            return;
        }
        let Some(array) = item.as_array_mut() else {
            return;
        };
        array
    } else {
        let item = doc
            .entry(table_key)
            .or_insert(Item::Value(Value::Array(Array::new())));
        if !item.is_array() {
            push_malformed_array_finding(table_key, item, merged, findings);
            *item = Item::Value(Value::Array(Array::new()));
        }
        let Some(array) = item.as_array_mut() else {
            return;
        };
        array
    };
    let mut current_paths = collect_current_paths(array);

    for (path, entry) in &merged.required {
        if glob_conflicts.required.contains(path) {
            continue;
        }
        let attribution = entry
            .collected
            .iter()
            .map(|(prov, _)| prov.clone())
            .collect::<Vec<_>>();
        apply_required(
            table_key,
            array,
            &mut current_paths,
            &entry.merged,
            &attribution,
            findings,
        );
    }

    for entry in merged.banned.values() {
        let path = &entry.merged.path;
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
        apply_banned(table_key, array, path, &message, &attribution, findings);
    }

    apply_forbidden_path_globs(table_key, array, globs, glob_conflicts, findings);

    if !merged.closed_by.is_empty() {
        let allowed = merged.required.keys().cloned().collect();
        let attribution = merged
            .closed_by
            .iter()
            .map(|(prov, _)| prov.clone())
            .collect::<Vec<_>>();
        prune_extras(table_key, array, &allowed, &attribution, findings);
    }
}

fn apply_forbidden_path_globs(
    table_key: &str,
    array: &mut Array,
    globs: &ResolvedForbiddenGlobRequirements<ClippyPathGlob>,
    glob_conflicts: &ClippyForbiddenGlobConflictBlocks,
    findings: &mut Vec<Finding>,
) {
    for (glob_identity, entry) in &globs.globs {
        if glob_conflicts.path_globs.contains(glob_identity) {
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
        let matcher = match compile_path_glob(glob) {
            Ok(matcher) => matcher,
            Err(error_message) => {
                findings.push(Finding::InvalidRequirements {
                    key: format!("{table_key}.{}", glob.glob),
                    message: error_message,
                    contributors: entry
                        .collected
                        .iter()
                        .map(|(prov, msg)| (prov.policy.clone(), msg.clone()))
                        .collect(),
                });
                continue;
            }
        };
        let mut removals = Vec::new();
        for (i, value) in array.iter().enumerate() {
            let Some(path) = read_entry_path(value) else {
                continue;
            };
            if matcher.is_match(&path) {
                removals.push((i, path));
            }
        }
        for (i, path) in removals.into_iter().rev() {
            let _ = array.remove(i);
            findings.push(Finding::Mismatch {
                key: format!("{table_key}[?path == \"{path}\"]"),
                current: Some(path),
                expected: "absent (path glob)".into(),
                message: message.clone(),
                severity: Severity::Error,
                attribution: attribution.clone(),
            });
        }
    }
}

fn apply_required(
    table_key: &str,
    array: &mut Array,
    current_paths: &mut BTreeSet<String>,
    entry: &BanEntry,
    attribution: &[Provenance],
    findings: &mut Vec<Finding>,
) {
    if let Some(index) = position_with_path(array, &entry.path) {
        let current = array.get(index).cloned();
        if current
            .as_ref()
            .is_some_and(|value| ban_value_matches(value, entry))
        {
            return;
        }
        let _ = array.replace(index, ban_value(entry));
        findings.push(Finding::Mismatch {
            key: format!("{table_key}[?path == \"{}\"]", entry.path),
            current: current.map(|value| value.to_string()),
            expected: format_entry(entry),
            message: entry.message.clone(),
            severity: Severity::Error,
            attribution: attribution.to_vec(),
        });
        return;
    }
    array.push(ban_value(entry));
    let _ = current_paths.insert(entry.path.clone());
    findings.push(Finding::Mismatch {
        key: format!("{table_key}[?path == \"{}\"]", entry.path),
        current: None,
        expected: format_entry(entry),
        message: entry.message.clone(),
        severity: Severity::Error,
        attribution: attribution.to_vec(),
    });
}

fn push_malformed_array_finding(
    table_key: &str,
    item: &Item,
    merged: &ResolvedItemRequirements<BanEntry>,
    findings: &mut Vec<Finding>,
) {
    let mut attribution = merged
        .required
        .values()
        .flat_map(|entry| entry.collected.iter().map(|(prov, _)| prov.clone()))
        .collect::<Vec<_>>();
    attribution.extend(
        merged
            .banned
            .values()
            .flat_map(|entry| entry.collected.iter().map(|(prov, _)| prov.clone())),
    );
    attribution.extend(merged.closed_by.iter().map(|(prov, _)| prov.clone()));
    findings.push(Finding::Mismatch {
        key: table_key.to_owned(),
        current: Some(item.to_string().trim().to_owned()),
        expected: "array".to_owned(),
        message: String::new(),
        severity: Severity::Error,
        attribution,
    });
}

fn apply_banned(
    table_key: &str,
    array: &mut Array,
    path_to_remove: &str,
    message: &str,
    attribution: &[Provenance],
    findings: &mut Vec<Finding>,
) {
    let positions = positions_with_path(array, path_to_remove);
    for i in positions.into_iter().rev() {
        let _ = array.remove(i);
        findings.push(Finding::Mismatch {
            key: format!("{table_key}[?path == \"{path_to_remove}\"]"),
            current: Some(path_to_remove.to_owned()),
            expected: "absent".into(),
            message: message.to_owned(),
            severity: Severity::Error,
            attribution: attribution.to_vec(),
        });
    }
}

fn prune_extras(
    table_key: &str,
    array: &mut Array,
    allowed: &BTreeSet<String>,
    attribution: &[Provenance],
    findings: &mut Vec<Finding>,
) {
    let mut indices_to_remove = Vec::new();
    for (i, value) in array.iter().enumerate() {
        if let Some(path) = read_entry_path(value) {
            if !allowed.contains(&path) {
                indices_to_remove.push((i, path));
            }
        }
    }
    for (i, path) in indices_to_remove.into_iter().rev() {
        let _ = array.remove(i);
        findings.push(Finding::Mismatch {
            key: format!("{table_key}[?path == \"{path}\"]"),
            current: Some(path),
            expected: "absent (closed table)".into(),
            message: String::new(),
            severity: Severity::Error,
            attribution: attribution.to_vec(),
        });
    }
}

fn collect_current_paths(array: &Array) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for entry in array {
        if let Some(path) = read_entry_path(entry) {
            let _ = out.insert(path);
        }
    }
    out
}

fn positions_with_path(array: &Array, target: &str) -> Vec<usize> {
    let mut out = Vec::new();
    for (i, value) in array.iter().enumerate() {
        if read_entry_path(value).as_deref() == Some(target) {
            out.push(i);
        }
    }
    out
}

fn position_with_path(array: &Array, target: &str) -> Option<usize> {
    array
        .iter()
        .enumerate()
        .find_map(|(i, value)| (read_entry_path(value).as_deref() == Some(target)).then_some(i))
}

fn read_entry_path(item: &Value) -> Option<String> {
    if let Some(path) = item.as_str() {
        return Some(path.to_owned());
    }
    if let Some(table) = item.as_inline_table() {
        return table
            .get("path")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned);
    }
    None
}

fn read_entry_reason(item: &Value) -> Option<String> {
    item.as_inline_table()
        .and_then(|table| table.get("reason").and_then(Value::as_str))
        .map(ToOwned::to_owned)
}

fn ban_value_matches(item: &Value, required: &BanEntry) -> bool {
    read_entry_path(item).as_deref() == Some(required.path.as_str())
        && (required.message.is_empty()
            || read_entry_reason(item).as_deref() == Some(required.message.as_str()))
}

fn ban_value(entry: &BanEntry) -> Value {
    // Required ban entries are writable array items.
    if entry.message.is_empty() {
        Value::from(entry.path.as_str())
    } else {
        let mut table = InlineTable::new();
        let _ = table.insert("path", Value::from(entry.path.as_str()));
        let _ = table.insert("reason", Value::from(entry.message.as_str()));
        Value::InlineTable(table)
    }
}

fn format_entry(entry: &BanEntry) -> String {
    if entry.message.is_empty() {
        format!("path={}", entry.path)
    } else {
        format!("path={} reason={}", entry.path, entry.message)
    }
}

fn compile_path_glob(glob: &ClippyPathGlob) -> Result<GlobMatcher, String> {
    GlobBuilder::new(&glob.glob)
        .build()
        .map(|glob| glob.compile_matcher())
        .map_err(|err| format!("invalid path glob `{}`: {err}", glob.glob))
}
