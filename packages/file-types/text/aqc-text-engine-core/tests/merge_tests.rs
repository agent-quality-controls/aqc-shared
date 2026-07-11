use aqc_file_engine_core::{ItemRequirements, Provenance, ScalarAssertion};
use aqc_text_engine_core::{TextFileContents, TextFileRequirements};

#[test]
fn merges_exact_and_contained_contents() -> Result<(), String> {
    let (resolved, conflicts) = TextFileRequirements::merge(vec![(
        provenance("policy"),
        TextFileRequirements {
            exact_contents: Some(ScalarAssertion::Equals(
                contents("abc")?,
                "exact".to_owned(),
            )),
            contents: required_contents("abc")?,
        },
    )]);

    assert!(conflicts.is_empty(), "A single policy must not conflict.");
    assert!(
        resolved.exact_contents.is_some(),
        "The exact contents assertion must resolve."
    );
    assert_eq!(
        resolved.contents.required.len(),
        1,
        "One contained item must resolve."
    );
    Ok(())
}

#[test]
fn conflicting_exact_contents_reports_conflict() -> Result<(), String> {
    let (_resolved, conflicts) = TextFileRequirements::merge(vec![
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
    ]);

    assert_eq!(
        conflicts.len(),
        1,
        "Different exact contents must conflict."
    );
    Ok(())
}

fn required_contents(value: &str) -> Result<ItemRequirements<TextFileContents>, String> {
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
