use aqc_file_engine_core::{ItemRequirements, Provenance, ScalarAssertion};
use aqc_text_engine_core::{TextFileContents, TextFileRequirements, TextSnippet, TextSnippetId};

#[test]
fn merges_exact_contents_and_snippets() -> Result<(), String> {
    let (resolved, conflicts) = TextFileRequirements::merge(vec![(
        provenance("policy"),
        TextFileRequirements {
            exact_contents: Some(ScalarAssertion::Equals(
                contents("abc")?,
                "exact".to_owned(),
            )),
            required_snippets: snippets(vec![snippet("chain", "abc")?]),
        },
    )]);

    assert!(conflicts.is_empty(), "single policy should not conflict");
    assert!(
        resolved.exact_contents.is_some(),
        "exact contents assertion should resolve"
    );
    assert_eq!(
        resolved.required_snippets.required.len(),
        1,
        "one snippet should resolve"
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
                required_snippets: ItemRequirements::default(),
            },
        ),
        (
            provenance("two"),
            TextFileRequirements {
                exact_contents: Some(ScalarAssertion::Equals(contents("two")?, "two".to_owned())),
                required_snippets: ItemRequirements::default(),
            },
        ),
    ]);

    assert_eq!(
        conflicts.len(),
        1,
        "different exact contents should conflict"
    );
    Ok(())
}

fn snippets(items: Vec<TextSnippet>) -> ItemRequirements<TextSnippet> {
    ItemRequirements {
        required: items
            .into_iter()
            .map(|item| (item, "snippet".to_owned()))
            .collect(),
        forbidden: Vec::new(),
        closed: None,
    }
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
