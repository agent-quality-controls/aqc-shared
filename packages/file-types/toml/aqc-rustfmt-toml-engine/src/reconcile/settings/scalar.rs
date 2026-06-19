//! Scalar setting reconciliation.

use aqc_file_engine_core::{ConfigScalar, Finding, Provenance, ScalarAssertion, Severity};
use toml_edit::{DocumentMut, Item, value as toml_value};

use super::toml_io::render_item;
/// Applies one resolved scalar assertion.
pub(super) fn apply_scalar(
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

/// Writes a scalar value when it does not equal the expected value.
fn apply_scalar_equals(
    doc: &mut DocumentMut,
    key: &str,
    want: &ConfigScalar,
    message: &str,
    attribution: &[Provenance],
    findings: &mut Vec<Finding>,
) {
    if doc.get(key).is_some_and(|item| scalar_matches(item, want)) {
        return;
    }
    findings.push(Finding::Mismatch {
        key: key.to_owned(),
        current: doc.get(key).and_then(render_item),
        expected: render_scalar(want),
        message: message.to_owned(),
        severity: Severity::Error,
        attribution: attribution.to_vec(),
    });
    doc[key] = scalar_item(want);
}

/// Reports when a scalar value is not one of the allowed values.
fn apply_scalar_one_of(
    doc: &DocumentMut,
    key: &str,
    allowed: &std::collections::BTreeSet<ConfigScalar>,
    message: &str,
    attribution: &[Provenance],
    findings: &mut Vec<Finding>,
) {
    if doc
        .get(key)
        .is_some_and(|item| allowed.iter().any(|want| scalar_matches(item, want)))
    {
        return;
    }
    let allowed = allowed.iter().map(render_scalar).collect::<Vec<_>>();
    findings.push(Finding::Mismatch {
        key: key.to_owned(),
        current: doc.get(key).and_then(render_item),
        expected: format!("one of {allowed:?}"),
        message: message.to_owned(),
        severity: Severity::Error,
        attribution: attribution.to_vec(),
    });
}

/// Reports when a scalar key is missing.
fn apply_scalar_present(
    doc: &DocumentMut,
    key: &str,
    message: &str,
    attribution: &[Provenance],
    findings: &mut Vec<Finding>,
) {
    if doc.contains_key(key) {
        return;
    }
    findings.push(Finding::Mismatch {
        key: key.to_owned(),
        current: None,
        expected: "present".to_owned(),
        message: message.to_owned(),
        severity: Severity::Error,
        attribution: attribution.to_vec(),
    });
}

/// Removes a scalar key that must be absent.
fn apply_scalar_absent(
    doc: &mut DocumentMut,
    key: &str,
    message: &str,
    attribution: &[Provenance],
    findings: &mut Vec<Finding>,
) {
    if !doc.contains_key(key) {
        return;
    }
    findings.push(Finding::Mismatch {
        key: key.to_owned(),
        current: doc.get(key).and_then(render_item),
        expected: "absent".to_owned(),
        message: message.to_owned(),
        severity: Severity::Error,
        attribution: attribution.to_vec(),
    });
    let _ = doc.as_table_mut().remove(key);
}

/// Returns whether a TOML item matches a scalar expectation.
pub(super) fn scalar_matches(item: &Item, want: &ConfigScalar) -> bool {
    match want {
        ConfigScalar::Str(expected) => item.as_str() == Some(expected.as_str()),
        ConfigScalar::Int(expected) => item.as_integer() == Some(*expected),
        ConfigScalar::Bool(expected) => item.as_bool() == Some(*expected),
    }
}

/// Converts a scalar expectation to a TOML item.
fn scalar_item(want: &ConfigScalar) -> Item {
    match want {
        ConfigScalar::Str(value) => toml_value(value.as_str()),
        ConfigScalar::Int(value) => toml_value(*value),
        ConfigScalar::Bool(value) => toml_value(*value),
    }
}

/// Renders a scalar expectation for findings.
fn render_scalar(want: &ConfigScalar) -> String {
    match want {
        ConfigScalar::Str(value) => format!("{value:?}"),
        ConfigScalar::Int(value) => value.to_string(),
        ConfigScalar::Bool(value) => value.to_string(),
    }
}
