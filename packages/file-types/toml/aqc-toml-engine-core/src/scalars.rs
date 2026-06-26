//! Shared TOML scalar helpers.

use std::collections::BTreeSet;

use aqc_file_engine_core::{ConfigScalar, Finding, Provenance, ScalarAssertion, Severity};
use toml_edit::{DocumentMut, Item, value as toml_value};

use crate::finding::push_mismatch;

/// Write/remove action needed to satisfy a scalar field assertion.
#[derive(Debug)]
pub enum ScalarFieldEdit {
    /// Write the item to the field.
    Write(Item),
    /// Remove the field.
    Remove,
}

/// Parse TOML bytes into a document, reporting invalid bytes or invalid TOML.
#[must_use]
#[expect(
    clippy::type_complexity,
    reason = "Returning the parsed document and findings is the engine contract shape."
)]
pub fn parse_or_report(
    current_bytes: Option<&[u8]>,
    file_label: &str,
) -> (DocumentMut, Vec<Finding>) {
    let mut findings = Vec::new();
    let text = match current_bytes {
        Some(bytes) => match std::str::from_utf8(bytes) {
            Ok(value) => value,
            Err(err) => {
                findings.push(Finding::ParseError {
                    message: format!("{file_label} is not valid UTF-8: {err}"),
                    severity: Severity::Error,
                });
                return (DocumentMut::new(), findings);
            }
        },
        None => "",
    };
    match text.parse::<DocumentMut>() {
        Ok(doc) => (doc, findings),
        Err(err) => {
            findings.push(Finding::ParseError {
                message: format!("{file_label} is not valid TOML: {err}"),
                severity: Severity::Error,
            });
            (DocumentMut::new(), findings)
        }
    }
}

/// Convert a config scalar into a TOML item for writing.
#[must_use]
pub fn scalar_item(scalar: &ConfigScalar) -> Item {
    match scalar {
        ConfigScalar::Str(value) => toml_value(value.clone()),
        ConfigScalar::Int(value) => toml_value(*value),
        ConfigScalar::Bool(value) => toml_value(*value),
    }
}

/// Return true when a TOML item equals the config scalar.
#[must_use]
pub fn scalar_matches(item: &Item, scalar: &ConfigScalar) -> bool {
    match scalar {
        ConfigScalar::Str(value) => item.as_str() == Some(value.as_str()),
        ConfigScalar::Int(value) => item.as_integer() == Some(*value),
        ConfigScalar::Bool(value) => item.as_bool() == Some(*value),
    }
}

/// Render a config scalar for finding output.
#[must_use]
pub fn render_scalar(scalar: &ConfigScalar) -> String {
    match scalar {
        ConfigScalar::Str(value) => value.clone(),
        ConfigScalar::Int(value) => value.to_string(),
        ConfigScalar::Bool(value) => value.to_string(),
    }
}

/// Render a TOML value item for finding output.
#[must_use]
pub fn render_item(item: &Item) -> Option<String> {
    item.as_value()
        .map(|value| value.to_string().trim().to_owned())
}

/// Apply one core scalar assertion to one top-level TOML key.
pub fn apply_scalar_assertion(
    doc: &mut DocumentMut,
    key: &str,
    assertion: &ScalarAssertion<ConfigScalar>,
    attribution: &[Provenance],
    findings: &mut Vec<Finding>,
) {
    match assertion {
        ScalarAssertion::Equals(want, message) => {
            apply_scalar_equals(doc, key, want, message, attribution, findings);
        }
        ScalarAssertion::OneOf(allowed, message) => {
            apply_scalar_one_of(doc, key, allowed, message, attribution, findings);
        }
        ScalarAssertion::Present(message) => {
            apply_scalar_present(doc, key, message, attribution, findings);
        }
        ScalarAssertion::Absent(message) => {
            apply_scalar_absent(doc, key, message, attribution, findings);
        }
        ScalarAssertion::AtLeast(..) | ScalarAssertion::AtMost(..) | ScalarAssertion::Range(..) => {
        }
    }
}

/// Decide the edit needed for one TOML scalar field and report mismatches.
pub fn scalar_field_edit(
    display_key: String,
    current: Option<&Item>,
    assertion: &ScalarAssertion<ConfigScalar>,
    attribution: &[Provenance],
    findings: &mut Vec<Finding>,
) -> Option<ScalarFieldEdit> {
    match assertion {
        ScalarAssertion::Equals(want, message) => {
            if current.is_some_and(|item| scalar_matches(item, want)) {
                return None;
            }
            push_mismatch(
                findings,
                display_key,
                current.and_then(render_item),
                render_scalar(want),
                message.to_owned(),
                attribution,
            );
            Some(ScalarFieldEdit::Write(scalar_item(want)))
        }
        ScalarAssertion::OneOf(allowed, message) => {
            if current.is_some_and(|item| allowed.iter().any(|want| scalar_matches(item, want))) {
                return None;
            }
            let rendered = allowed.iter().map(render_scalar).collect::<Vec<_>>();
            push_mismatch(
                findings,
                display_key,
                current.and_then(render_item),
                format!("one of {rendered:?}"),
                message.to_owned(),
                attribution,
            );
            None
        }
        ScalarAssertion::Present(message) => {
            if current.is_some() {
                return None;
            }
            push_mismatch(
                findings,
                display_key,
                None,
                "present".to_owned(),
                message.to_owned(),
                attribution,
            );
            None
        }
        ScalarAssertion::Absent(message) => {
            let rendered = current.and_then(render_item)?;
            push_mismatch(
                findings,
                display_key,
                Some(rendered),
                "absent".to_owned(),
                message.to_owned(),
                attribution,
            );
            Some(ScalarFieldEdit::Remove)
        }
        ScalarAssertion::AtLeast(..) | ScalarAssertion::AtMost(..) | ScalarAssertion::Range(..) => {
            None
        }
    }
}

/// Return true when one assertion fails against an optional TOML item.
#[must_use]
pub fn scalar_assertion_fails(
    current: Option<&Item>,
    assertion: &ScalarAssertion<ConfigScalar>,
) -> bool {
    match assertion {
        ScalarAssertion::Equals(want, _) => !current.is_some_and(|item| scalar_matches(item, want)),
        ScalarAssertion::OneOf(allowed, _) => {
            !current.is_some_and(|item| allowed.iter().any(|allowed| scalar_matches(item, allowed)))
        }
        ScalarAssertion::Present(_) => current.is_none(),
        ScalarAssertion::Absent(_) => current.is_some(),
        ScalarAssertion::AtLeast(..) | ScalarAssertion::AtMost(..) | ScalarAssertion::Range(..) => {
            true
        }
    }
}

/// Applies an `Equals` scalar assertion to a top-level TOML key.
fn apply_scalar_equals(
    doc: &mut DocumentMut,
    key: &str,
    want: &ConfigScalar,
    message: &str,
    attribution: &[Provenance],
    findings: &mut Vec<Finding>,
) {
    if let Some(ScalarFieldEdit::Write(item)) = scalar_field_edit(
        key.to_owned(),
        doc.get(key),
        &ScalarAssertion::Equals(want.clone(), message.to_owned()),
        attribution,
        findings,
    ) {
        doc[key] = item;
    }
}

/// Reports a `OneOf` scalar assertion on a top-level TOML key without choosing a value.
fn apply_scalar_one_of(
    doc: &DocumentMut,
    key: &str,
    allowed: &BTreeSet<ConfigScalar>,
    message: &str,
    attribution: &[Provenance],
    findings: &mut Vec<Finding>,
) {
    let _ = scalar_field_edit(
        key.to_owned(),
        doc.get(key),
        &ScalarAssertion::OneOf(allowed.clone(), message.to_owned()),
        attribution,
        findings,
    );
}

/// Reports a missing top-level TOML scalar key.
fn apply_scalar_present(
    doc: &DocumentMut,
    key: &str,
    message: &str,
    attribution: &[Provenance],
    findings: &mut Vec<Finding>,
) {
    let _ = scalar_field_edit(
        key.to_owned(),
        doc.get(key),
        &ScalarAssertion::Present(message.to_owned()),
        attribution,
        findings,
    );
}

/// Removes a top-level TOML scalar key when an `Absent` assertion requires it.
fn apply_scalar_absent(
    doc: &mut DocumentMut,
    key: &str,
    message: &str,
    attribution: &[Provenance],
    findings: &mut Vec<Finding>,
) {
    if matches!(
        scalar_field_edit(
            key.to_owned(),
            doc.get(key),
            &ScalarAssertion::Absent(message.to_owned()),
            attribution,
            findings,
        ),
        Some(ScalarFieldEdit::Remove)
    ) {
        let _ = doc.remove(key);
    }
}
