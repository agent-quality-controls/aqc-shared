use aqc_file_engine_core::{
    Finding, Provenance, ResolvedRequirement, ScalarAssertion, ScalarValue, Severity,
    render_scalar_assertion, scalar_assertion_matches, scalar_assertion_writable_value,
};

use crate::{ParsedYamlMapping, YamlFieldValue};

pub trait YamlScalar: ScalarValue {
    fn decode_yaml(value: &YamlFieldValue) -> Option<Self>;
    fn write_yaml(document: &ParsedYamlMapping, key: &str, value: &Self);
}

impl YamlScalar for bool {
    fn decode_yaml(value: &YamlFieldValue) -> Option<Self> {
        match value {
            YamlFieldValue::Boolean(value) => Some(*value),
            YamlFieldValue::String(_)
            | YamlFieldValue::Integer(_)
            | YamlFieldValue::StringSequence(_)
            | YamlFieldValue::StringBooleanMapping(_) => None,
        }
    }

    fn write_yaml(document: &ParsedYamlMapping, key: &str, value: &Self) {
        document.set_boolean(key, *value);
    }
}

impl YamlScalar for u64 {
    fn decode_yaml(value: &YamlFieldValue) -> Option<Self> {
        match value {
            YamlFieldValue::Integer(value) => Some(*value),
            YamlFieldValue::String(_)
            | YamlFieldValue::Boolean(_)
            | YamlFieldValue::StringSequence(_)
            | YamlFieldValue::StringBooleanMapping(_) => None,
        }
    }

    fn write_yaml(document: &ParsedYamlMapping, key: &str, value: &Self) {
        document.set_integer(key, *value);
    }
}

pub fn apply_scalar_assertion<T: YamlScalar>(
    document: &ParsedYamlMapping,
    key: &str,
    requirement: Option<&ResolvedRequirement<ScalarAssertion<T>, ScalarAssertion<T>>>,
    findings: &mut Vec<Finding>,
) {
    let Some(requirement) = requirement else {
        return;
    };
    let field = document.field(key);
    let current = field
        .as_ref()
        .ok()
        .and_then(|value| value.as_ref())
        .and_then(T::decode_yaml);
    let exists = field
        .as_ref()
        .ok()
        .and_then(|value| value.as_ref())
        .is_some();
    if field.is_ok() && scalar_assertion_matches(&requirement.merged, current.as_ref(), exists) {
        return;
    }
    findings.push(Finding::Mismatch {
        key: key.to_owned(),
        selector: None,
        current: current.as_ref().map(ScalarValue::render),
        expected: render_scalar_assertion(&requirement.merged),
        message: requirement.merged.message().to_owned(),
        severity: Severity::Error,
        attribution: requirement
            .collected
            .iter()
            .map(|(provenance, _)| provenance.clone())
            .collect::<Vec<Provenance>>(),
    });
    match scalar_assertion_writable_value(&requirement.merged) {
        Some(value) => T::write_yaml(document, key, value),
        None if matches!(requirement.merged, ScalarAssertion::Absent(_)) => {
            let _ = document.remove_if_effectively_absent(key);
        }
        None => {}
    }
}
