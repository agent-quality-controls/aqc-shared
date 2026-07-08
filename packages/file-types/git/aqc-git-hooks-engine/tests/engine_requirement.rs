use std::path::{Path, PathBuf};

use aqc_text_file_engine as _;

use aqc_file_engine_core::{
    Engine, EngineFileState, EngineRequirement, Finding, ItemRequirements, Provenance,
    ScalarAssertion,
};
use aqc_git_hooks_engine::{
    GitHooksEngine, GitHooksRequirements, TextFileContents, TextFilePath, TextFileRequirement,
    TextSnippet, TextSnippetId,
};

#[test]
fn git_hooks_requirement_routes_to_text_engine() -> Result<(), String> {
    let requirements = GitHooksRequirements {
        files: aqc_git_hooks_engine::TextFileRequirements {
            files: ItemRequirements {
                required: vec![(
                    TextFileRequirement {
                        path: TextFilePath::new(".githooks/pre-commit")
                            .map_err(|err| err.to_string())?,
                        exact_contents: None,
                        required_snippets: ItemRequirements {
                            required: vec![(
                                TextSnippet {
                                    id: TextSnippetId::new("runner")
                                        .map_err(|err| err.to_string())?,
                                    contents: TextFileContents::new(
                                        b"shakrs validate workspace\n".to_vec(),
                                    )
                                    .map_err(|err| err.to_string())?,
                                },
                                "hook snippet required".to_owned(),
                            )],
                            forbidden: Vec::new(),
                            closed: None,
                        },
                        executable: Some(ScalarAssertion::Equals(
                            true,
                            "hook must be executable".to_owned(),
                        )),
                    },
                    "hook file required".to_owned(),
                )],
                forbidden: Vec::new(),
                closed: None,
            },
        },
    };
    let reqs: Vec<(Provenance, Box<dyn EngineRequirement>)> =
        vec![(provenance("hooks"), Box::new(requirements))];
    let output = GitHooksEngine.reconcile(
        Path::new("/repo"),
        &[EngineFileState {
            path: PathBuf::from("/repo/.githooks/pre-commit"),
            bytes: Some(b"#!/bin/sh\n".to_vec()),
            executable: Some(false),
        }],
        &reqs,
    );

    assert_eq!(
        output.files[0].path,
        PathBuf::from("/repo/.githooks/pre-commit")
    );
    assert!(
        output.files[0]
            .findings
            .iter()
            .any(|finding| matches!(finding, Finding::Mismatch { key, .. } if key == ".githooks/pre-commit.snippet.runner")),
        "missing git hook snippet should be reported"
    );
    assert_eq!(output.files[0].expected_executable, Some(true));
    Ok(())
}

fn provenance(policy: &str) -> Provenance {
    Provenance {
        policy: policy.to_owned(),
    }
}
