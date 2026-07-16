//! Reconciliation for explicit effective root-key membership.

use std::collections::BTreeSet;

use aqc_file_engine_core::{
    Finding, KeyedItem, Provenance, ResolvedExactItems, ResolvedItemRequirements, Severity,
    item_presence_difference,
};

use crate::ParsedYamlMapping;

/// Remove and report present effective root keys rejected by membership.
pub fn remove_rejected_effective_root_keys(
    document: &ParsedYamlMapping,
    requirements: &ResolvedItemRequirements<KeyedItem<()>>,
    findings: &mut Vec<Finding>,
) -> Option<BTreeSet<String>> {
    let Ok(effective_keys) = document.effective_keys() else {
        findings.push(Finding::Mismatch {
            key: "<<".to_owned(),
            selector: None,
            current: Some("invalid merge source".to_owned()),
            expected: "resolvable effective root keys".to_owned(),
            message: membership_message(requirements),
            severity: Severity::Error,
            attribution: membership_attribution(requirements),
        });
        return None;
    };
    let current_keys = effective_keys.into_iter().collect::<BTreeSet<_>>();
    let difference = item_presence_difference(&current_keys, requirements);
    let mut rejected = BTreeSet::new();

    for (key, resolved) in difference.forbidden {
        let _ = rejected.insert(key.clone());
        findings.push(Finding::Mismatch {
            key: key.clone(),
            selector: None,
            current: Some("present".to_owned()),
            expected: "absent".to_owned(),
            message: resolved
                .collected
                .first()
                .map_or_else(String::new, |(_, message)| message.clone()),
            severity: Severity::Error,
            attribution: resolved.attribution(),
        });
        let _ = document.remove_if_effectively_absent(key);
    }
    if let Some(exact) = &requirements.exact {
        for key in difference.unexpected {
            let _ = rejected.insert(key.clone());
            findings.push(Finding::Mismatch {
                key: key.clone(),
                selector: None,
                current: Some("present".to_owned()),
                expected: "absent (exact keys)".to_owned(),
                message: exact_message(exact),
                severity: Severity::Error,
                attribution: exact_attribution(exact),
            });
            let _ = document.remove_if_effectively_absent(key);
        }
    }
    Some(rejected)
}

/// Report effective root keys still missing after child reconciliation.
pub fn report_missing_effective_root_keys(
    document: &ParsedYamlMapping,
    requirements: &ResolvedItemRequirements<KeyedItem<()>>,
    findings: &mut Vec<Finding>,
) {
    let Ok(effective_keys) = document.effective_keys() else {
        return;
    };
    let current_keys = effective_keys.into_iter().collect::<BTreeSet<_>>();
    let difference = item_presence_difference(&current_keys, requirements);

    for (key, resolved) in difference.missing {
        findings.push(Finding::UnwritableRequiredKey {
            key: key.clone(),
            expected: "present root key".to_owned(),
            attribution: resolved.attribution(),
        });
    }
}

fn membership_message(requirements: &ResolvedItemRequirements<KeyedItem<()>>) -> String {
    let required = requirements
        .required
        .values()
        .flat_map(|resolved| resolved.collected.iter().map(|(_, (_, message))| message))
        .next()
        .cloned();
    let forbidden = requirements
        .forbidden
        .values()
        .flat_map(|resolved| resolved.collected.iter().map(|(_, message)| message))
        .next()
        .cloned();
    required
        .or(forbidden)
        .or_else(|| requirements.exact.as_ref().map(exact_message))
        .unwrap_or_default()
}

fn membership_attribution(
    requirements: &ResolvedItemRequirements<KeyedItem<()>>,
) -> Vec<Provenance> {
    let mut attribution = requirements
        .required
        .values()
        .flat_map(aqc_file_engine_core::ResolvedRequirement::attribution)
        .chain(
            requirements
                .forbidden
                .values()
                .flat_map(aqc_file_engine_core::ResolvedRequirement::attribution),
        )
        .chain(requirements.exact.iter().flat_map(exact_attribution))
        .collect::<Vec<_>>();
    attribution.sort();
    attribution.dedup();
    attribution
}

fn exact_message(exact: &ResolvedExactItems<KeyedItem<()>>) -> String {
    exact
        .collected
        .first()
        .map_or_else(String::new, |(_, (_, message))| message.clone())
}

fn exact_attribution(exact: &ResolvedExactItems<KeyedItem<()>>) -> Vec<Provenance> {
    exact
        .collected
        .iter()
        .map(|(provenance, _)| provenance.clone())
        .collect()
}
