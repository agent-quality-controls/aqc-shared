//! Reconciliation for clippy.toml disallowed tables.

#![cfg_attr(
    not(test),
    expect(
        clippy::missing_docs_in_private_items,
        reason = "Private disallowed-table helpers are internal reconciliation steps."
    )
)]

use aqc_file_engine_core::{
    Finding, ResolvedForbiddenGlobRequirements, ResolvedItemRequirements, Severity,
};
use aqc_toml_engine_core::{
    TomlArrayItem, TomlItemError, TomlItemField, reconcile_array_items,
    report_list_shape_with_message,
};
use globset::{GlobBuilder, GlobMatcher};
use toml_edit::{Array, DocumentMut, InlineTable, Value};

use crate::requirement::{ClippyForbiddenGlobConflictBlocks, ClippyPathGlob, DisallowedEntry};

pub(crate) fn apply(
    doc: &mut DocumentMut,
    table_key: &str,
    merged: &ResolvedItemRequirements<DisallowedEntry>,
    globs: &ResolvedForbiddenGlobRequirements<ClippyPathGlob>,
    glob_conflicts: &ClippyForbiddenGlobConflictBlocks,
    findings: &mut Vec<Finding>,
) {
    if merged.required.is_empty()
        && merged.forbidden.is_empty()
        && merged.exact.is_none()
        && globs.globs.is_empty()
    {
        return;
    }

    reconcile_array_items(
        doc,
        TomlItemField::new(&[], table_key, table_key),
        merged,
        findings,
    );

    let Some(item) = doc.get_mut(table_key) else {
        return;
    };
    let Some(array) = item.as_array_mut() else {
        if !globs.globs.is_empty() {
            let attribution = globs
                .globs
                .values()
                .flat_map(|entry| entry.collected.iter().map(|(prov, _)| prov.clone()))
                .collect::<Vec<_>>();
            let message = globs
                .globs
                .values()
                .flat_map(|entry| entry.collected.iter().map(|(_, msg)| msg.as_str()))
                .next()
                .unwrap_or_default()
                .to_owned();
            let _ = report_list_shape_with_message(doc, table_key, message, &attribution, findings);
        }
        return;
    };
    apply_forbidden_path_globs(table_key, array, globs, glob_conflicts, findings);
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

fn disallowed_value(entry: &DisallowedEntry) -> Value {
    if entry.message.is_empty() {
        Value::from(entry.path.as_str())
    } else {
        let mut table = InlineTable::new();
        let _ = table.insert("path", Value::from(entry.path.as_str()));
        let _ = table.insert("reason", Value::from(entry.message.as_str()));
        Value::InlineTable(table)
    }
}

fn format_entry(entry: &DisallowedEntry) -> String {
    if entry.message.is_empty() {
        format!("path={}", entry.path)
    } else {
        format!("path={} reason={}", entry.path, entry.message)
    }
}

impl TomlArrayItem for DisallowedEntry {
    fn read_value(value: &Value) -> Result<Self, TomlItemError> {
        let Some(path) = read_entry_path(value) else {
            return Err(TomlItemError::new(
                "disallowed entry requires a string path or inline table path",
            ));
        };
        Ok(Self {
            path,
            message: read_entry_reason(value).unwrap_or_default(),
        })
    }

    fn write_value(&self) -> Value {
        disallowed_value(self)
    }

    fn matches_value(current: &Self, required: &Self) -> bool {
        current.path == required.path
            && (required.message.is_empty() || current.message == required.message)
    }

    fn render_value(&self) -> String {
        format_entry(self)
    }
}

fn compile_path_glob(glob: &ClippyPathGlob) -> Result<GlobMatcher, String> {
    GlobBuilder::new(&glob.glob)
        .build()
        .map(|glob| glob.compile_matcher())
        .map_err(|err| format!("invalid path glob `{}`: {err}", glob.glob))
}
