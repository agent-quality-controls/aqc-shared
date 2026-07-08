//! Text file reconciliation.

use std::path::Path;

use aqc_file_engine_core::{
    EngineFileOutput, EngineFileState, Finding, ItemRequirements, Provenance, ScalarAssertion,
    Severity,
};

use crate::requirement::{
    ResolvedTextFileRequirements, TextFileContents, TextFileRequirement, TextSnippet,
};

pub fn reconcile_text_files(
    root: &Path,
    current: &[EngineFileState],
    requirements: &ResolvedTextFileRequirements,
) -> Vec<EngineFileOutput> {
    requirements
        .files
        .required
        .values()
        .map(|entry| reconcile_file(root, current, &entry.merged, &entry.collected))
        .collect()
}

fn reconcile_file(
    root: &Path,
    current: &[EngineFileState],
    requirement: &TextFileRequirement,
    attribution: &[(Provenance, (TextFileRequirement, String))],
) -> EngineFileOutput {
    let path = root.join(requirement.path.as_path());
    let state = current.iter().find(|state| state.path == path);
    let current_bytes = state.and_then(|state| state.bytes.as_deref());
    let mut findings = Vec::new();
    let mut expected = current_bytes.map(<[u8]>::to_vec).unwrap_or_default();

    if let Some(assertion) = &requirement.exact_contents {
        apply_exact_contents(
            requirement.path.as_path().display().to_string().as_str(),
            assertion,
            current_bytes,
            attribution,
            &mut expected,
            &mut findings,
        );
    }

    apply_required_snippets(
        requirement.path.as_path().display().to_string().as_str(),
        &requirement.required_snippets,
        requirement.exact_contents.is_some(),
        current_bytes,
        attribution,
        &mut expected,
        &mut findings,
    );

    let expected_executable = requirement.executable.as_ref().and_then(desired_executable);
    if let Some(expected_mode) = expected_executable {
        let current_mode = state.and_then(|state| state.executable);
        if current_mode != Some(expected_mode) {
            findings.push(Finding::Mismatch {
                key: "executable".to_owned(),
                current: current_mode.map(|value| value.to_string()),
                expected: expected_mode.to_string(),
                message: requirement
                    .executable
                    .as_ref()
                    .map_or_else(String::new, |assertion| assertion.message().to_owned()),
                severity: Severity::Error,
                attribution: attribution.iter().map(|(prov, _)| prov.clone()).collect(),
            });
        }
    }

    EngineFileOutput {
        path,
        expected_bytes: expected,
        expected_executable,
        findings,
    }
}

fn apply_exact_contents(
    key: &str,
    assertion: &ScalarAssertion<TextFileContents>,
    current_bytes: Option<&[u8]>,
    attribution: &[(Provenance, (TextFileRequirement, String))],
    expected: &mut Vec<u8>,
    findings: &mut Vec<Finding>,
) {
    if let ScalarAssertion::Equals(contents, message) = assertion {
        *expected = contents.as_bytes().to_vec();
        if current_bytes != Some(contents.as_bytes()) {
            findings.push(Finding::Mismatch {
                key: key.to_owned(),
                current: current_bytes.map(|bytes| format!("{} bytes", bytes.len())),
                expected: format!("{} bytes", contents.as_bytes().len()),
                message: message.clone(),
                severity: Severity::Error,
                attribution: attribution.iter().map(|(prov, _)| prov.clone()).collect(),
            });
        }
    }
}

fn apply_required_snippets(
    key: &str,
    snippets: &ItemRequirements<TextSnippet>,
    exact_mode: bool,
    current_bytes: Option<&[u8]>,
    attribution: &[(Provenance, (TextFileRequirement, String))],
    expected: &mut Vec<u8>,
    findings: &mut Vec<Finding>,
) {
    for (snippet_req, message) in &snippets.required {
        let snippet = snippet_req.contents.as_bytes();
        if exact_mode && !contains_bytes(expected, snippet) {
            findings.push(Finding::InvalidRequirements {
                key: format!("{key}.snippet.{}", snippet_req.id.as_str()),
                message: "exact contents must contain required snippet".to_owned(),
                contributors: attribution
                    .iter()
                    .map(|(prov, _)| (prov.policy.clone(), message.clone()))
                    .collect(),
            });
        }
        if !contains_bytes(current_bytes.unwrap_or_default(), snippet) {
            findings.push(Finding::Mismatch {
                key: format!("{key}.snippet.{}", snippet_req.id.as_str()),
                current: current_bytes.map(|bytes| format!("{} bytes", bytes.len())),
                expected: "snippet present".to_owned(),
                message: message.clone(),
                severity: Severity::Error,
                attribution: attribution.iter().map(|(prov, _)| prov.clone()).collect(),
            });
            if !exact_mode && !contains_bytes(expected, snippet) {
                expected.extend_from_slice(snippet);
            }
        }
    }
}

fn desired_executable(assertion: &ScalarAssertion<bool>) -> Option<bool> {
    match assertion {
        ScalarAssertion::Equals(value, _)
        | ScalarAssertion::AtLeast(value, _)
        | ScalarAssertion::AtMost(value, _) => Some(*value),
        ScalarAssertion::Range(value, _, _) => Some(*value),
        ScalarAssertion::Absent(_) => Some(false),
        ScalarAssertion::Present(_) | ScalarAssertion::OneOf(_, _) => None,
    }
}

fn contains_bytes(haystack: &[u8], needle: &[u8]) -> bool {
    !needle.is_empty()
        && haystack
            .windows(needle.len())
            .any(|window| window == needle)
}
