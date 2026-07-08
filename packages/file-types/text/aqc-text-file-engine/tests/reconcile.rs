use std::path::{Path, PathBuf};

use aqc_file_engine_core::{
    Engine, EngineFileState, EngineRequirement, Finding, ItemRequirements, Provenance,
    ScalarAssertion,
};
use aqc_text_file_engine::{
    TextFileContents, TextFileEngine, TextFilePath, TextFileRequirement, TextFileRequirements,
    TextSnippet, TextSnippetId,
};

#[test]
fn exact_contents_mismatch_reports() -> Result<(), String> {
    let output = reconcile(
        requirements(text_file(
            "hooks/pre-commit",
            Some("expected\n"),
            Vec::new(),
            None,
        )?),
        vec![EngineFileState {
            path: PathBuf::from("/repo/hooks/pre-commit"),
            bytes: Some(b"current\n".to_vec()),
            executable: None,
        }],
    );

    assert!(
        output.files[0].findings.iter().any(
            |finding| matches!(finding, Finding::Mismatch { key, .. } if key == "hooks/pre-commit")
        ),
        "exact content mismatch should be reported"
    );
    assert_eq!(output.files[0].expected_bytes, b"expected\n".to_vec());
    Ok(())
}

#[test]
fn missing_snippet_reports() -> Result<(), String> {
    let output = reconcile(
        requirements(text_file(
            "hooks/pre-commit",
            None,
            vec![snippet("runner", "cargo test\n")?],
            None,
        )?),
        vec![EngineFileState {
            path: PathBuf::from("/repo/hooks/pre-commit"),
            bytes: Some(b"#!/bin/sh\n".to_vec()),
            executable: None,
        }],
    );

    assert!(
        output.files[0]
            .findings
            .iter()
            .any(|finding| matches!(finding, Finding::Mismatch { key, .. } if key == "hooks/pre-commit.snippet.runner")),
        "missing snippet should be reported"
    );
    assert!(
        output.files[0]
            .expected_bytes
            .windows(b"cargo test\n".len())
            .any(|window| window == b"cargo test\n"),
        "init bytes should add the missing snippet"
    );
    Ok(())
}

#[test]
fn missing_executable_reports() -> Result<(), String> {
    let output = reconcile(
        requirements(text_file(
            "hooks/pre-commit",
            Some("#!/bin/sh\n"),
            Vec::new(),
            Some(true),
        )?),
        vec![EngineFileState {
            path: PathBuf::from("/repo/hooks/pre-commit"),
            bytes: Some(b"#!/bin/sh\n".to_vec()),
            executable: Some(false),
        }],
    );

    assert!(
        output.files[0]
            .findings
            .iter()
            .any(|finding| matches!(finding, Finding::Mismatch { key, .. } if key == "executable")),
        "missing executable bit should be reported"
    );
    assert_eq!(output.files[0].expected_executable, Some(true));
    Ok(())
}

#[test]
fn init_writes_expected_bytes() -> Result<(), String> {
    let output = reconcile(
        requirements(text_file(
            "hooks/pre-commit",
            None,
            vec![
                snippet("header", "#!/bin/sh\n")?,
                snippet("runner", "cargo test\n")?,
            ],
            Some(true),
        )?),
        Vec::new(),
    );

    assert_eq!(
        output.files[0].expected_bytes,
        b"#!/bin/sh\ncargo test\n".to_vec()
    );
    assert_eq!(output.files[0].expected_executable, Some(true));
    Ok(())
}

fn reconcile(
    requirements: TextFileRequirements,
    current: Vec<EngineFileState>,
) -> aqc_file_engine_core::EngineOutput {
    let reqs: Vec<(Provenance, Box<dyn EngineRequirement>)> =
        vec![(provenance("policy"), Box::new(requirements))];
    TextFileEngine.reconcile(Path::new("/repo"), &current, &reqs)
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
