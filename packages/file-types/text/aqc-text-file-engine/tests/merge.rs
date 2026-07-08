use aqc_file_engine_core::{ItemRequirements, Provenance, ScalarAssertion};
use aqc_text_file_engine::{
    TextFileContents, TextFilePath, TextFileRequirement, TextFileRequirements, TextSnippet,
    TextSnippetId,
};

#[test]
fn uses_core_item_merge_for_files() -> Result<(), String> {
    let first = text_file("hooks/pre-commit", Some("one\n"), Vec::new(), None)?;
    let second = text_file("hooks/pre-commit", Some("two\n"), Vec::new(), None)?;
    let (resolved, conflicts) = TextFileRequirements::merge(vec![
        (provenance("alpha"), requirements(first)),
        (provenance("beta"), requirements(second)),
    ]);

    assert_eq!(
        resolved.files.required.len(),
        1,
        "conflicting nested fields should keep the file item"
    );
    assert!(
        resolved
            .files
            .required
            .values()
            .all(|entry| entry.merged.exact_contents.is_none()),
        "conflicting exact contents should be dropped from the merged file"
    );
    assert_eq!(
        conflicts.len(),
        1,
        "file identity merge should report one conflict"
    );
    assert_eq!(conflicts[0].key, "files.hooks/pre-commit.exact_contents");
    Ok(())
}

#[test]
fn uses_core_item_merge_for_snippets() -> Result<(), String> {
    let first = text_file(
        "hooks/pre-commit",
        None,
        vec![snippet("runner", "cargo test\n")?],
        None,
    )?;
    let second = text_file(
        "hooks/pre-commit",
        None,
        vec![snippet("runner", "cargo clippy\n")?],
        None,
    )?;
    let (resolved, conflicts) = TextFileRequirements::merge(vec![
        (provenance("alpha"), requirements(first)),
        (provenance("beta"), requirements(second)),
    ]);

    assert_eq!(
        resolved.files.required.len(),
        1,
        "conflicting nested snippets should keep the file item"
    );
    assert!(
        resolved.files.required.values().all(|entry| entry
            .merged
            .required_snippets
            .required
            .is_empty()),
        "conflicting snippets should be dropped from the merged file"
    );
    assert_eq!(
        conflicts[0].key, "files.hooks/pre-commit.required_snippets.runner",
        "snippet identity merge should report the nested snippet key"
    );
    Ok(())
}

fn requirements(file: TextFileRequirement) -> TextFileRequirements {
    TextFileRequirements {
        files: ItemRequirements {
            required: vec![(file, "file required".to_owned())],
            forbidden: Vec::new(),
            closed: None,
        },
    }
}

fn text_file(
    path: &str,
    exact_contents: Option<&str>,
    snippets: Vec<TextSnippet>,
    executable: Option<bool>,
) -> Result<TextFileRequirement, String> {
    let exact_contents = exact_contents
        .map(|contents| TextFileContents::new(contents.as_bytes().to_vec()))
        .transpose()
        .map_err(|err| err.to_string())?
        .map(|contents| ScalarAssertion::Equals(contents, "exact contents".to_owned()));
    let executable =
        executable.map(|value| ScalarAssertion::Equals(value, "executable".to_owned()));
    Ok(TextFileRequirement {
        path: TextFilePath::new(path).map_err(|err| err.to_string())?,
        exact_contents,
        required_snippets: ItemRequirements {
            required: snippets
                .into_iter()
                .map(|snippet| (snippet, "snippet required".to_owned()))
                .collect(),
            forbidden: Vec::new(),
            closed: None,
        },
        executable,
    })
}

fn snippet(id: &str, contents: &str) -> Result<TextSnippet, String> {
    Ok(TextSnippet {
        id: TextSnippetId::new(id).map_err(|err| err.to_string())?,
        contents: TextFileContents::new(contents.as_bytes().to_vec())
            .map_err(|err| err.to_string())?,
    })
}

fn provenance(policy: &str) -> Provenance {
    Provenance {
        policy: policy.to_owned(),
    }
}
