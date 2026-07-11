use aqc_file_engine_core::{Finding, ItemRequirements, Provenance, ScalarAssertion};
use aqc_text_engine_core::{TextFileContents, TextFileRequirements, reconcile_text_file};

#[test]
fn exact_contents_reports_mismatch_and_expected_bytes() -> Result<(), String> {
    let output = reconcile(requirements_with_exact("expected\n")?, Some(b"old\n"));

    assert_eq!(
        output.expected_bytes,
        b"expected\n".to_vec(),
        "Exact contents must become the expected bytes."
    );
    assert!(
        output.findings.iter().any(
            |finding| matches!(finding, Finding::Mismatch { key, .. } if key == "exact_contents")
        ),
        "Different bytes must report an exact-content mismatch."
    );
    Ok(())
}

#[test]
fn contained_contents_are_appended_once() -> Result<(), String> {
    let requirements = requirements_with_contents("required\n")?;
    let first = reconcile(requirements.clone(), Some(b"old\n"));
    let second = reconcile(requirements, Some(&first.expected_bytes));

    assert_eq!(
        first.expected_bytes,
        b"old\nrequired\n".to_vec(),
        "Missing contents must be appended."
    );
    assert_eq!(
        second.expected_bytes, first.expected_bytes,
        "Contained-content init must be idempotent."
    );
    assert!(
        first
            .findings
            .iter()
            .any(|finding| matches!(finding, Finding::Mismatch { key, .. } if key == "contents")),
        "Missing contents must report a mismatch."
    );
    Ok(())
}

#[test]
fn exact_contents_must_contain_required_contents() -> Result<(), String> {
    let mut requirements = requirements_with_exact("exact\n")?;
    requirements.contents = item_requirements("missing\n")?;
    let output = reconcile(requirements, Some(b"old\n"));

    assert!(
        output
            .findings
            .iter()
            .any(|finding| matches!(finding, Finding::InvalidRequirements { key, .. } if key == "contents")),
        "Exact contents that omit required contents must be invalid."
    );
    Ok(())
}

#[test]
fn unsupported_exact_operation_is_invalid() {
    let requirements = TextFileRequirements {
        exact_contents: Some(ScalarAssertion::Present("present".to_owned())),
        contents: ItemRequirements::default(),
    };
    let output = reconcile(requirements, Some(b"old\n"));

    assert!(
        output.findings.iter().any(
            |finding| matches!(finding, Finding::InvalidRequirements { key, .. } if key == "exact_contents")
        ),
        "Unsupported scalar operations must be invalid."
    );
}

#[test]
fn unsupported_exact_operation_is_not_hidden_by_equals() -> Result<(), String> {
    let contents = TextFileContents::new(b"exact\n".to_vec()).map_err(|error| error.to_string())?;
    let first = TextFileRequirements {
        exact_contents: Some(ScalarAssertion::Equals(
            contents.clone(),
            "equals".to_owned(),
        )),
        contents: ItemRequirements::default(),
    };
    let second = TextFileRequirements {
        exact_contents: Some(ScalarAssertion::Present("present".to_owned())),
        contents: ItemRequirements::default(),
    };
    let (resolved, conflicts) = TextFileRequirements::merge(vec![
        (provenance("first"), first),
        (provenance("second"), second),
    ]);
    assert!(
        conflicts.is_empty(),
        "compatible scalar assertions must merge"
    );

    let output = reconcile_text_file(Some(contents.as_bytes()), &resolved);
    assert!(
        output.findings.iter().any(
            |finding| matches!(finding, Finding::InvalidRequirements { key, .. } if key == "exact_contents")
        ),
        "Unsupported collected operations must remain invalid after merge."
    );
    Ok(())
}

#[test]
fn omitted_contents_preserve_each_contributor_message() -> Result<(), String> {
    let required = contents("required\n")?;
    let exact = contents("exact\n")?;
    let make = |message: &str| TextFileRequirements {
        exact_contents: Some(ScalarAssertion::Equals(exact.clone(), "exact".to_owned())),
        contents: ItemRequirements {
            required: vec![(required.clone(), message.to_owned())],
            forbidden: Vec::new(),
            exact: None,
        },
    };
    let (resolved, conflicts) = TextFileRequirements::merge(vec![
        (provenance("first"), make("first message")),
        (provenance("second"), make("second message")),
    ]);
    assert!(conflicts.is_empty(), "matching requirements must merge");

    let output = reconcile_text_file(Some(exact.as_bytes()), &resolved);
    let contributors = output.findings.iter().find_map(|finding| {
        if let Finding::InvalidRequirements {
            key, contributors, ..
        } = finding
            && key == "contents"
        {
            return Some(contributors);
        }
        None
    });
    assert_eq!(
        contributors,
        Some(&vec![
            ("first".to_owned(), "first message".to_owned()),
            ("second".to_owned(), "second message".to_owned()),
        ]),
        "Each policy must retain its own diagnostic message."
    );
    Ok(())
}

fn reconcile(
    requirements: TextFileRequirements,
    current: Option<&[u8]>,
) -> aqc_file_engine_core::EngineOutput {
    let (resolved, conflicts) =
        TextFileRequirements::merge(vec![(provenance("policy"), requirements)]);
    assert!(conflicts.is_empty(), "Test requirements must resolve.");
    reconcile_text_file(current, &resolved)
}

fn requirements_with_exact(value: &str) -> Result<TextFileRequirements, String> {
    Ok(TextFileRequirements {
        exact_contents: Some(ScalarAssertion::Equals(
            contents(value)?,
            "exact".to_owned(),
        )),
        contents: ItemRequirements::default(),
    })
}

fn requirements_with_contents(value: &str) -> Result<TextFileRequirements, String> {
    Ok(TextFileRequirements {
        exact_contents: None,
        contents: item_requirements(value)?,
    })
}

fn item_requirements(value: &str) -> Result<ItemRequirements<TextFileContents>, String> {
    Ok(ItemRequirements {
        required: vec![(contents(value)?, "contents".to_owned())],
        forbidden: Vec::new(),
        exact: None,
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
