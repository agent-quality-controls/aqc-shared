use aqc_file_engine_core::{ItemRequirements, Provenance, ScalarAssertion};
use aqc_text_file_engine::{TextFileContents, TextFileRequirements};

#[test]
fn merges_exact_and_contained_contents() -> Result<(), String> {
    let resolved = TextFileRequirements::merge(vec![(
        provenance("policy"),
        TextFileRequirements {
            exact_contents: Some(ScalarAssertion::Equals(
                contents("abc")?,
                "exact".to_owned(),
            )),
            contents: required_contents("abc")?,
        },
    )])
    .map_err(|conflicts| format!("unexpected conflicts: {conflicts:?}"))?;

    assert!(
        resolved.exact_contents().is_some(),
        "The exact contents assertion must resolve."
    );
    assert_eq!(
        resolved.contents().required.len(),
        1,
        "One contained item must resolve."
    );
    Ok(())
}

#[test]
fn conflicting_exact_contents_reports_conflict() -> Result<(), String> {
    let conflicts = TextFileRequirements::merge(vec![
        (
            provenance("one"),
            TextFileRequirements {
                exact_contents: Some(ScalarAssertion::Equals(contents("one")?, "one".to_owned())),
                contents: ItemRequirements::default(),
            },
        ),
        (
            provenance("two"),
            TextFileRequirements {
                exact_contents: Some(ScalarAssertion::Equals(contents("two")?, "two".to_owned())),
                contents: ItemRequirements::default(),
            },
        ),
    ])
    .expect_err("different exact contents must not expose resolved requirements");

    assert_eq!(
        conflicts.len(),
        1,
        "Different exact contents must conflict."
    );
    let Some(conflict) = conflicts.first() else {
        return Ok(());
    };
    assert_eq!(conflict.key, "exact_contents");
    assert_eq!(conflict.reason, "scalar-disagree");
    assert_eq!(
        conflict
            .contributors
            .iter()
            .map(|(provenance, value)| (provenance.policy.as_str(), value.as_str()))
            .collect::<Vec<_>>(),
        vec![("one", "equals 3 bytes"), ("two", "equals 3 bytes")]
    );
    Ok(())
}

fn required_contents(value: &str) -> Result<ItemRequirements<TextFileContents>, String> {
    Ok(ItemRequirements {
        required: vec![(contents(value)?, "contents".to_owned())],
        forbidden: Vec::new(),
        allowed: None,
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
