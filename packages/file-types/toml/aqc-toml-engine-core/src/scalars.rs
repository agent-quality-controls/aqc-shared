//! Shared TOML scalar helpers.

use aqc_file_engine_core::{
    ConfigScalar, Finding, Provenance, ScalarAssertion, Severity, render_scalar_assertion,
    scalar_assertion_matches, scalar_assertion_writable_value,
};
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
#[allow(
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
    decode_scalar(item).as_ref() == Some(scalar)
}

fn decode_scalar(item: &Item) -> Option<ConfigScalar> {
    item.as_str()
        .map(|value| ConfigScalar::Str(value.to_owned()))
        .or_else(|| item.as_integer().map(ConfigScalar::Int))
        .or_else(|| item.as_bool().map(ConfigScalar::Bool))
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
    match scalar_field_edit(
        key.to_owned(),
        doc.get(key),
        assertion,
        attribution,
        findings,
    ) {
        Some(ScalarFieldEdit::Write(item)) => doc[key] = item,
        Some(ScalarFieldEdit::Remove) => {
            let _ = doc.remove(key);
        }
        None => {}
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
    let decoded = current.and_then(decode_scalar);
    if scalar_assertion_matches(assertion, decoded.as_ref(), current.is_some()) {
        return None;
    }
    push_mismatch(
        findings,
        display_key,
        current.and_then(render_item),
        render_scalar_assertion(assertion),
        assertion.message().to_owned(),
        attribution,
    );
    scalar_assertion_writable_value(assertion)
        .map(|value| ScalarFieldEdit::Write(scalar_item(value)))
        .or_else(|| {
            matches!(assertion, ScalarAssertion::Absent(_)).then_some(ScalarFieldEdit::Remove)
        })
}

/// Return true when one assertion fails against an optional TOML item.
#[must_use]
pub fn scalar_assertion_fails(
    current: Option<&Item>,
    assertion: &ScalarAssertion<ConfigScalar>,
) -> bool {
    let decoded = current.and_then(decode_scalar);
    !scalar_assertion_matches(assertion, decoded.as_ref(), current.is_some())
}
