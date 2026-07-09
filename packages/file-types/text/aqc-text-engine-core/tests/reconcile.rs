use aqc_file_engine_core::{Finding, ItemRequirements, Provenance, ScalarAssertion};
use aqc_text_engine_core::{
    TextFileContents, TextFileRequirements, TextSnippet, TextSnippetId, reconcile_text_file,
};

#[test]
fn exact_contents_reports_mismatch_and_expected_bytes() -> Result<(), String> {
    let output = reconcile(requirements_with_exact("expected\n")?, Some(b"old\n"));

    assert_eq!(
        output.expected_bytes,
        b"expected\n".to_vec(),
        "exact contents should be expected bytes"
    );
    assert!(
        output.findings.iter().any(
            |finding| matches!(finding, Finding::Mismatch { key, .. } if key == "exact_contents")
        ),
        "different bytes should report exact content mismatch"
    );
    Ok(())
}

#[test]
fn snippet_is_appended_when_missing() -> Result<(), String> {
    let output = reconcile(
        requirements_with_snippet("chain", "snippet\n")?,
        Some(b"old\n"),
    );

    assert_eq!(
        output.expected_bytes,
        b"old\nsnippet\n".to_vec(),
        "missing snippet should be appended"
    );
    assert!(
        output
            .findings
            .iter()
            .any(|finding| matches!(finding, Finding::Mismatch { key, .. } if key == "required_snippets.chain")),
        "missing snippet should report mismatch"
    );
    Ok(())
}

#[test]
fn exact_contents_must_contain_required_snippet() -> Result<(), String> {
    let mut req = requirements_with_exact("exact\n")?;
    req.required_snippets = snippet_requirements("chain", "missing\n")?;
    let output = reconcile(req, Some(b"old\n"));

    assert!(
        output
            .findings
            .iter()
            .any(|finding| matches!(finding, Finding::InvalidRequirements { key, .. } if key == "required_snippets.chain")),
        "exact contents that omit a required snippet should be invalid"
    );
    Ok(())
}

fn reconcile(
    requirements: TextFileRequirements,
    current: Option<&[u8]>,
) -> aqc_file_engine_core::EngineOutput {
    let (resolved, conflicts) =
        TextFileRequirements::merge(vec![(provenance("policy"), requirements)]);
    assert!(conflicts.is_empty(), "test requirements should resolve");
    reconcile_text_file(current, &resolved)
}

fn requirements_with_exact(value: &str) -> Result<TextFileRequirements, String> {
    Ok(TextFileRequirements {
        exact_contents: Some(ScalarAssertion::Equals(
            contents(value)?,
            "exact".to_owned(),
        )),
        required_snippets: ItemRequirements::default(),
    })
}

fn requirements_with_snippet(id: &str, value: &str) -> Result<TextFileRequirements, String> {
    Ok(TextFileRequirements {
        exact_contents: None,
        required_snippets: snippet_requirements(id, value)?,
    })
}

fn snippet_requirements(id: &str, value: &str) -> Result<ItemRequirements<TextSnippet>, String> {
    Ok(ItemRequirements {
        required: vec![(snippet(id, value)?, "snippet".to_owned())],
        forbidden: Vec::new(),
        closed: None,
    })
}

fn snippet(id: &str, value: &str) -> Result<TextSnippet, String> {
    Ok(TextSnippet {
        id: TextSnippetId::new(id).map_err(|err| err.to_string())?,
        contents: contents(value)?,
    })
}

fn contents(value: &str) -> Result<TextFileContents, String> {
    TextFileContents::new(value.as_bytes().to_vec()).map_err(|err| err.to_string())
}

fn provenance(policy: &str) -> Provenance {
    Provenance {
        policy: policy.to_owned(),
    }
}
