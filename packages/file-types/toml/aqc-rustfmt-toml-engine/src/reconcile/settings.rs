//! Reconcile top-level `rustfmt.toml` settings.

use std::collections::BTreeSet;

use aqc_file_engine_core::{
    ConfigScalar, Finding, Provenance, ResolvedListRequirements, ResolvedRequirement, Severity,
};
use toml_edit::{Array, DocumentMut, Item, Value, value as toml_value};

use crate::requirement::{
    ResolvedRustfmtScalarAssertion, ResolvedRustfmtTomlRequirements, RustfmtScalarAssertion,
};

pub(crate) fn apply(
    doc: &mut DocumentMut,
    requirement: &ResolvedRustfmtTomlRequirements,
    findings: &mut Vec<Finding>,
) {
    for (setting, resolved) in &requirement.scalar_settings {
        let key = setting.file_key();
        let attribution = scalar_attribution_for(doc, key, resolved);
        apply_scalar(doc, key, &resolved.merged, &attribution, findings);
    }
    for (setting, resolved) in &requirement.list_settings {
        apply_list(doc, setting.file_key(), resolved, findings);
    }
    apply_closed(doc, requirement, findings);
}

fn scalar_attribution_for(
    doc: &DocumentMut,
    key: &str,
    resolved: &ResolvedRequirement<ResolvedRustfmtScalarAssertion, RustfmtScalarAssertion>,
) -> Vec<Provenance> {
    let current = doc.get(key);
    let filtered = resolved
        .collected
        .iter()
        .filter(|(_, assertion)| scalar_assertion_fails(current, assertion))
        .map(|(prov, _)| prov.clone())
        .collect::<Vec<_>>();
    if filtered.is_empty() {
        attribution(resolved)
    } else {
        filtered
    }
}

fn scalar_assertion_fails(current: Option<&Item>, assertion: &RustfmtScalarAssertion) -> bool {
    match assertion {
        RustfmtScalarAssertion::Equals(want, _) => {
            !current.is_some_and(|item| scalar_matches(item, want))
        }
        RustfmtScalarAssertion::OneOf(allowed, _) => !current
            .and_then(Item::as_str)
            .is_some_and(|value| allowed.contains(value)),
        RustfmtScalarAssertion::Present(_) => current.is_none(),
        RustfmtScalarAssertion::Absent(_) => current.is_some(),
    }
}

fn apply_scalar(
    doc: &mut DocumentMut,
    key: &str,
    assertion: &ResolvedRustfmtScalarAssertion,
    attribution: &[Provenance],
    findings: &mut Vec<Finding>,
) {
    match assertion {
        ResolvedRustfmtScalarAssertion::Equals(want, message) => {
            apply_scalar_equals(doc, key, want, message, attribution, findings);
        }
        ResolvedRustfmtScalarAssertion::OneOf(allowed, message) => {
            apply_scalar_one_of(doc, key, allowed, message, attribution, findings);
        }
        ResolvedRustfmtScalarAssertion::Present(message) => {
            apply_scalar_present(doc, key, message, attribution, findings);
        }
        ResolvedRustfmtScalarAssertion::Absent(message) => {
            apply_scalar_absent(doc, key, message, attribution, findings);
        }
    }
}

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

fn apply_scalar_one_of(
    doc: &DocumentMut,
    key: &str,
    allowed: &BTreeSet<String>,
    message: &str,
    attribution: &[Provenance],
    findings: &mut Vec<Finding>,
) {
    if doc
        .get(key)
        .and_then(Item::as_str)
        .is_some_and(|value| allowed.contains(value))
    {
        return;
    }
    findings.push(Finding::Mismatch {
        key: key.to_owned(),
        current: doc.get(key).and_then(render_item),
        expected: format!("one of {allowed:?}"),
        message: message.to_owned(),
        severity: Severity::Error,
        attribution: attribution.to_vec(),
    });
}

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

fn apply_list(
    doc: &mut DocumentMut,
    key: &str,
    requirements: &ResolvedListRequirements,
    findings: &mut Vec<Finding>,
) {
    if report_list_shape(doc, key, requirements, findings) {
        let values = list_values(doc, key);
        write_list(doc, key, &values);
    }
    for (item, resolved) in &requirements.contains {
        let attribution = attribution(resolved);
        let message = resolved
            .collected
            .first()
            .map(|(_, msg)| msg.as_str())
            .unwrap_or_default();
        if list_values(doc, key).iter().any(|current| current == item) {
            continue;
        }
        findings.push(Finding::Mismatch {
            key: format!("{key}.{item}"),
            current: Some(render_list(doc, key)),
            expected: format!("list containing {item}"),
            message: message.to_owned(),
            severity: Severity::Error,
            attribution,
        });
        let mut values = list_values(doc, key);
        values.push(item.clone());
        write_list(doc, key, &values);
    }
    for (item, resolved) in &requirements.excludes {
        let attribution = attribution(resolved);
        let message = resolved
            .collected
            .first()
            .map(|(_, msg)| msg.as_str())
            .unwrap_or_default();
        let values = list_values(doc, key);
        if !values.iter().any(|current| current == item) {
            continue;
        }
        findings.push(Finding::Mismatch {
            key: format!("{key}.{item}"),
            current: Some(render_list(doc, key)),
            expected: format!("list excluding {item}"),
            message: message.to_owned(),
            severity: Severity::Error,
            attribution,
        });
        let kept = values
            .into_iter()
            .filter(|current| current != item)
            .collect::<Vec<_>>();
        write_list(doc, key, &kept);
    }
    if let Some(exact) = &requirements.exact {
        let attribution = attribution(exact);
        let message = exact
            .collected
            .first()
            .map(|(_, (_, msg))| msg.as_str())
            .unwrap_or_default();
        if list_values(doc, key) == exact.merged {
            return;
        }
        findings.push(Finding::Mismatch {
            key: key.to_owned(),
            current: Some(render_list(doc, key)),
            expected: format!("{:?}", exact.merged),
            message: message.to_owned(),
            severity: Severity::Error,
            attribution,
        });
        write_list(doc, key, &exact.merged);
    }
}

fn report_list_shape(
    doc: &DocumentMut,
    key: &str,
    requirements: &ResolvedListRequirements,
    findings: &mut Vec<Finding>,
) -> bool {
    let Some(item) = doc.get(key) else {
        return false;
    };
    let message = list_message(requirements);
    let attribution = list_attribution(requirements);
    let Some(array) = item.as_array() else {
        findings.push(Finding::Mismatch {
            key: key.to_owned(),
            current: render_item(item),
            expected: "array of strings".to_owned(),
            message,
            severity: Severity::Error,
            attribution,
        });
        return true;
    };
    let mut malformed = false;
    for (index, value) in array.iter().enumerate() {
        if value.as_str().is_some() {
            continue;
        }
        malformed = true;
        findings.push(Finding::Mismatch {
            key: format!("{key}[{index}]"),
            current: Some(value.to_string()),
            expected: "string".to_owned(),
            message: message.clone(),
            severity: Severity::Error,
            attribution: attribution.clone(),
        });
    }
    malformed
}

fn list_message(requirements: &ResolvedListRequirements) -> String {
    requirements
        .contains
        .values()
        .flat_map(|resolved| resolved.collected.iter().map(|(_, msg)| msg.as_str()))
        .chain(
            requirements
                .excludes
                .values()
                .flat_map(|resolved| resolved.collected.iter().map(|(_, msg)| msg.as_str())),
        )
        .chain(
            requirements
                .exact
                .iter()
                .flat_map(|resolved| resolved.collected.iter().map(|(_, (_, msg))| msg.as_str())),
        )
        .next()
        .unwrap_or_default()
        .to_owned()
}

fn list_attribution(requirements: &ResolvedListRequirements) -> Vec<Provenance> {
    requirements
        .contains
        .values()
        .flat_map(|resolved| resolved.collected.iter().map(|(prov, _)| prov.clone()))
        .chain(
            requirements
                .excludes
                .values()
                .flat_map(|resolved| resolved.collected.iter().map(|(prov, _)| prov.clone())),
        )
        .chain(
            requirements
                .exact
                .iter()
                .flat_map(|resolved| resolved.collected.iter().map(|(prov, _)| prov.clone())),
        )
        .collect()
}

fn apply_closed(
    doc: &mut DocumentMut,
    requirement: &ResolvedRustfmtTomlRequirements,
    findings: &mut Vec<Finding>,
) {
    if requirement.closed_settings.is_empty() {
        return;
    }
    let allowed = requirement
        .scalar_settings
        .keys()
        .map(|key| key.file_key())
        .chain(requirement.list_settings.keys().map(|key| key.file_key()))
        .collect::<BTreeSet<_>>();
    let extras = doc
        .as_table()
        .iter()
        .map(|(key, _)| key.to_owned())
        .filter(|key| !allowed.contains(key.as_str()))
        .collect::<Vec<_>>();
    for extra in extras {
        findings.push(Finding::Mismatch {
            key: extra.clone(),
            current: doc.get(&extra).and_then(render_item),
            expected: "absent because rustfmt.toml settings are closed".to_owned(),
            message: requirement
                .closed_settings
                .first()
                .map(|(_, msg)| msg.clone())
                .unwrap_or_default(),
            severity: Severity::Error,
            attribution: requirement
                .closed_settings
                .iter()
                .map(|(prov, _)| prov.clone())
                .collect(),
        });
        let _ = doc.as_table_mut().remove(&extra);
    }
}

fn scalar_matches(item: &Item, want: &ConfigScalar) -> bool {
    match want {
        ConfigScalar::Str(expected) => item.as_str() == Some(expected.as_str()),
        ConfigScalar::Int(expected) => item.as_integer() == Some(*expected),
        ConfigScalar::Bool(expected) => item.as_bool() == Some(*expected),
    }
}

fn scalar_item(want: &ConfigScalar) -> Item {
    match want {
        ConfigScalar::Str(value) => toml_value(value.as_str()),
        ConfigScalar::Int(value) => toml_value(*value),
        ConfigScalar::Bool(value) => toml_value(*value),
    }
}

fn render_scalar(want: &ConfigScalar) -> String {
    match want {
        ConfigScalar::Str(value) => format!("{value:?}"),
        ConfigScalar::Int(value) => value.to_string(),
        ConfigScalar::Bool(value) => value.to_string(),
    }
}

fn render_item(item: &Item) -> Option<String> {
    item.as_value().map(ToString::to_string)
}

fn list_values(doc: &DocumentMut, key: &str) -> Vec<String> {
    doc.get(key)
        .and_then(Item::as_array)
        .map(|array| {
            array
                .iter()
                .filter_map(Value::as_str)
                .map(ToOwned::to_owned)
                .collect()
        })
        .unwrap_or_default()
}

fn write_list(doc: &mut DocumentMut, key: &str, values: &[String]) {
    let mut array = Array::default();
    for item in values {
        array.push(item.as_str());
    }
    doc[key] = toml_value(array);
}

fn render_list(doc: &DocumentMut, key: &str) -> String {
    doc.get(key)
        .and_then(render_item)
        .unwrap_or_else(|| "[]".to_owned())
}

fn attribution<Merged, Assertion>(
    resolved: &ResolvedRequirement<Merged, Assertion>,
) -> Vec<Provenance> {
    resolved
        .collected
        .iter()
        .map(|(prov, _)| prov.clone())
        .collect()
}
