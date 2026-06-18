//! List setting reconciliation.

use aqc_file_engine_core::{Finding, Provenance, ResolvedListRequirements, Severity};
use toml_edit::DocumentMut;

use super::toml_io::{attribution, list_values, render_item, render_list, write_list};

pub(super) fn apply_list(
    doc: &mut DocumentMut,
    key: &str,
    requirements: &ResolvedListRequirements,
    findings: &mut Vec<Finding>,
) {
    if report_list_shape(doc, key, requirements, findings) {
        let values = list_values(doc, key);
        write_list(doc, key, &values);
    }
    apply_list_contains(doc, key, requirements, findings);
    apply_list_excludes(doc, key, requirements, findings);
    apply_list_exact(doc, key, requirements, findings);
}

fn apply_list_contains(
    doc: &mut DocumentMut,
    key: &str,
    requirements: &ResolvedListRequirements,
    findings: &mut Vec<Finding>,
) {
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
}

fn apply_list_excludes(
    doc: &mut DocumentMut,
    key: &str,
    requirements: &ResolvedListRequirements,
    findings: &mut Vec<Finding>,
) {
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
}

fn apply_list_exact(
    doc: &mut DocumentMut,
    key: &str,
    requirements: &ResolvedListRequirements,
    findings: &mut Vec<Finding>,
) {
    let Some(exact) = &requirements.exact else {
        return;
    };
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
