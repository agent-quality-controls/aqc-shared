//! Text byte-stream reconciliation.

use aqc_file_engine_core::{EngineOutput, Finding, Provenance, ScalarAssertion, Severity};

use crate::requirement::{ResolvedTextFileRequirements, TextFileContents, TextSnippet};

pub fn reconcile_text_file(
    current_bytes: Option<&[u8]>,
    requirements: &ResolvedTextFileRequirements,
) -> EngineOutput {
    let mut findings = Vec::new();
    let mut expected = current_bytes.map(<[u8]>::to_vec).unwrap_or_default();

    if let Some(assertion) = &requirements.exact_contents {
        apply_exact_contents(
            &assertion.merged,
            current_bytes,
            &assertion
                .collected
                .iter()
                .map(|(prov, assertion)| (prov.clone(), assertion.clone()))
                .collect::<Vec<_>>(),
            &mut expected,
            &mut findings,
        );
    }

    apply_required_snippets(
        &requirements.required_snippets,
        requirements.exact_contents.is_some(),
        current_bytes,
        &mut expected,
        &mut findings,
    );

    EngineOutput {
        expected_bytes: expected,
        findings,
    }
}

/// Apply an exact-content assertion and report a byte-count mismatch when the
/// current bytes differ.
fn apply_exact_contents(
    assertion: &ScalarAssertion<TextFileContents>,
    current_bytes: Option<&[u8]>,
    attribution: &[(Provenance, ScalarAssertion<TextFileContents>)],
    expected: &mut Vec<u8>,
    findings: &mut Vec<Finding>,
) {
    if let ScalarAssertion::Equals(contents, message) = assertion {
        *expected = contents.as_bytes().to_vec();
        if current_bytes != Some(contents.as_bytes()) {
            findings.push(Finding::Mismatch {
                key: "exact_contents".to_owned(),
                current: current_bytes.map(|bytes| format!("{} bytes", bytes.len())),
                expected: format!("{} bytes", contents.as_bytes().len()),
                message: message.clone(),
                severity: Severity::Error,
                attribution: attribution.iter().map(|(prov, _)| prov.clone()).collect(),
            });
        }
    }
}

/// Apply required snippets against current bytes and append missing snippets
/// when exact-content mode is not active.
fn apply_required_snippets(
    snippets: &aqc_file_engine_core::ResolvedItemRequirements<TextSnippet>,
    exact_mode: bool,
    current_bytes: Option<&[u8]>,
    expected: &mut Vec<u8>,
    findings: &mut Vec<Finding>,
) {
    for entry in snippets.required.values() {
        let snippet = entry.merged.contents.as_bytes();
        let message = entry
            .collected
            .first()
            .map_or_else(String::new, |(_, (_, msg))| msg.clone());
        if exact_mode && !contains_bytes(expected, snippet) {
            findings.push(Finding::InvalidRequirements {
                key: format!("required_snippets.{}", entry.merged.id.as_str()),
                message: "exact contents must contain required snippet".to_owned(),
                contributors: entry
                    .collected
                    .iter()
                    .map(|(prov, _)| (prov.policy.clone(), message.clone()))
                    .collect(),
            });
        }
        if !contains_bytes(current_bytes.unwrap_or_default(), snippet) {
            findings.push(Finding::Mismatch {
                key: format!("required_snippets.{}", entry.merged.id.as_str()),
                current: current_bytes.map(|bytes| format!("{} bytes", bytes.len())),
                expected: "snippet present".to_owned(),
                message,
                severity: Severity::Error,
                attribution: entry
                    .collected
                    .iter()
                    .map(|(prov, _)| prov.clone())
                    .collect(),
            });
            if !exact_mode && !contains_bytes(expected, snippet) {
                expected.extend_from_slice(snippet);
            }
        }
    }
}

/// Return whether `needle` appears as a contiguous byte sequence in `haystack`.
fn contains_bytes(haystack: &[u8], needle: &[u8]) -> bool {
    !needle.is_empty()
        && haystack
            .windows(needle.len())
            .any(|window| window == needle)
}
