//! Text byte-stream reconciliation.

use aqc_file_engine_core::{EngineOutput, Finding, ScalarAssertion, Severity};

use crate::requirement::{ResolvedTextFileRequirements, TextFileContents};

pub fn reconcile_text_file(
    current_bytes: Option<&[u8]>,
    requirements: &ResolvedTextFileRequirements,
) -> EngineOutput {
    let mut findings = Vec::new();
    let mut expected = current_bytes.map(<[u8]>::to_vec).unwrap_or_default();

    if let Some(assertion) = &requirements.exact_contents {
        apply_exact_contents(assertion, current_bytes, &mut expected, &mut findings);
    }
    reject_unsupported_contents(requirements, &mut findings);
    apply_required_contents(
        &requirements.contents,
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

/// Apply the supported exact byte assertion or report an unsupported operation.
fn apply_exact_contents(
    assertion: &aqc_file_engine_core::ResolvedRequirement<
        ScalarAssertion<TextFileContents>,
        ScalarAssertion<TextFileContents>,
    >,
    current_bytes: Option<&[u8]>,
    expected: &mut Vec<u8>,
    findings: &mut Vec<Finding>,
) {
    if assertion
        .collected
        .iter()
        .any(|(_, item)| !matches!(item, ScalarAssertion::Equals(..)))
    {
        findings.push(Finding::InvalidRequirements {
            key: "exact_contents".to_owned(),
            message: "text exact contents require Equals".to_owned(),
            contributors: assertion
                .collected
                .iter()
                .map(|(prov, item)| (prov.policy.clone(), item.message().to_owned()))
                .collect(),
        });
        return;
    }
    let ScalarAssertion::Equals(contents, message) = &assertion.merged else {
        findings.push(Finding::InvalidRequirements {
            key: "exact_contents".to_owned(),
            message: "text exact contents require Equals".to_owned(),
            contributors: assertion
                .collected
                .iter()
                .map(|(prov, item)| (prov.policy.clone(), item.message().to_owned()))
                .collect(),
        });
        return;
    };
    *expected = contents.as_bytes().to_vec();
    if current_bytes != Some(contents.as_bytes()) {
        findings.push(Finding::Mismatch {
            key: "exact_contents".to_owned(),
            selector: None,
            current: current_bytes.map(|bytes| format!("{} bytes", bytes.len())),
            expected: format!("{} bytes", contents.as_bytes().len()),
            message: message.clone(),
            severity: Severity::Error,
            attribution: assertion
                .collected
                .iter()
                .map(|(prov, _)| prov.clone())
                .collect(),
        });
    }
}

/// Report item operations that text containment cannot reconcile or initialize.
fn reject_unsupported_contents(
    requirements: &ResolvedTextFileRequirements,
    findings: &mut Vec<Finding>,
) {
    for entry in requirements.contents.forbidden.values() {
        findings.push(Finding::InvalidRequirements {
            key: "contents".to_owned(),
            message: "text contained contents do not support forbidden items".to_owned(),
            contributors: entry
                .collected
                .iter()
                .map(|(prov, message)| (prov.policy.clone(), message.clone()))
                .collect(),
        });
    }
    if let Some(exact) = &requirements.contents.exact {
        findings.push(Finding::InvalidRequirements {
            key: "contents".to_owned(),
            message: "text contained contents do not support exact collections".to_owned(),
            contributors: exact
                .collected
                .iter()
                .map(|(prov, (_, message))| (prov.policy.clone(), message.clone()))
                .collect(),
        });
    }
}

/// Validate required byte sequences and append each missing sequence once.
fn apply_required_contents(
    contents: &aqc_file_engine_core::ResolvedItemRequirements<TextFileContents>,
    exact_mode: bool,
    current_bytes: Option<&[u8]>,
    expected: &mut Vec<u8>,
    findings: &mut Vec<Finding>,
) {
    for entry in contents.required.values() {
        let required = entry.merged.as_bytes();
        let message = entry
            .collected
            .first()
            .map_or_else(String::new, |(_, (_, message))| message.clone());
        if exact_mode && !contains_bytes(expected, required) {
            findings.push(Finding::InvalidRequirements {
                key: "contents".to_owned(),
                message: "exact contents must contain required contents".to_owned(),
                contributors: entry
                    .collected
                    .iter()
                    .map(|(prov, (_, contributor_message))| {
                        (prov.policy.clone(), contributor_message.clone())
                    })
                    .collect(),
            });
        }
        if !contains_bytes(current_bytes.unwrap_or_default(), required) {
            findings.push(Finding::Mismatch {
                key: "contents".to_owned(),
                selector: None,
                current: current_bytes.map(|bytes| format!("{} bytes", bytes.len())),
                expected: "required contents present".to_owned(),
                message,
                severity: Severity::Error,
                attribution: entry
                    .collected
                    .iter()
                    .map(|(prov, _)| prov.clone())
                    .collect(),
            });
            if !exact_mode && !contains_bytes(expected, required) {
                expected.extend_from_slice(required);
            }
        }
    }
}

/// Return whether `needle` occurs as one contiguous byte sequence.
fn contains_bytes(haystack: &[u8], needle: &[u8]) -> bool {
    !needle.is_empty()
        && haystack
            .windows(needle.len())
            .any(|window| window == needle)
}
