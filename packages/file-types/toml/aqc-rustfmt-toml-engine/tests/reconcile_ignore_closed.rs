use aqc_toml_engine_core as _;
use std::collections::BTreeMap;

use aqc_file_engine_core::{
    ConfigScalar, FileEngine, Finding, ForbiddenGlobRequirements, ListRequirements, Provenance,
    ScalarAssertion,
};
use aqc_rustfmt_toml_engine::{
    ResolvedRustfmtTomlRequirements, RustfmtIgnorePathGlob, RustfmtListSetting,
    RustfmtScalarSetting, RustfmtTomlEngine, RustfmtTomlRequirements,
};
use globset as _;
use toml_edit as _;

type IgnoreGlobCases<'a> = Vec<(&'a str, &'a str)>;

#[test]
fn forbidden_ignore_path_glob_removes_matching_values() {
    let output = reconcile(
        "ignore = [\"target/generated\", \"src/lib.rs\"]\n",
        RustfmtTomlRequirements {
            forbidden_ignore_path_globs: ignore_globs(vec![(
                "target/**",
                "do not disable formatting under target",
            )]),
            ..RustfmtTomlRequirements::default()
        },
    );

    let expected = String::from_utf8(first_bytes(&output)).unwrap_or_default();
    assert!(
        !expected.contains("target/generated"),
        "matching ignore value should be removed"
    );
    assert!(
        expected.contains("src/lib.rs"),
        "non-matching ignore value should remain"
    );
    assert!(
        output.findings.iter().any(
            |finding| matches!(finding, Finding::Mismatch { key, expected: finding_expected, .. } if key == "ignore.target/generated" && finding_expected == "absent (path glob)")
        ),
        "matching ignore value should report a path-glob mismatch"
    );
}

#[test]
fn invalid_forbidden_ignore_path_glob_reports_invalid_requirements() {
    let output = reconcile(
        "ignore = [\"target/generated\"]\n",
        RustfmtTomlRequirements {
            forbidden_ignore_path_globs: ignore_globs(vec![("[", "bad glob")]),
            ..RustfmtTomlRequirements::default()
        },
    );

    assert!(
        output.findings.iter().any(
            |finding| matches!(finding, Finding::InvalidRequirements { key, message, .. } if key == "ignore.[" && message.contains("invalid ignore path glob"))
        ),
        "invalid glob syntax should report invalid requirements"
    );
}

#[test]
fn forbidden_ignore_path_glob_reports_and_normalizes_malformed_ignore_list() {
    let output = reconcile(
        "ignore = [\"target/generated\", 42]\n",
        RustfmtTomlRequirements {
            forbidden_ignore_path_globs: ignore_globs(vec![(
                "unmatched/**",
                "glob requires ignore list shape",
            )]),
            ..RustfmtTomlRequirements::default()
        },
    );

    let expected = String::from_utf8(first_bytes(&output)).unwrap_or_default();
    assert!(
        expected.contains("ignore = [\"target/generated\"]"),
        "malformed ignore list should be normalized when glob rules inspect it"
    );
    assert!(
        output.findings.iter().any(
            |finding| matches!(finding, Finding::Mismatch { key, expected: finding_expected, .. } if key == "ignore[1]" && finding_expected == "string")
        ),
        "non-string ignore item should report shape finding"
    );
}

#[test]
fn closed_settings_keep_ignore_when_only_glob_rules_manage_it() {
    let output = reconcile_resolved(
        "ignore = [\"src/lib.rs\"]\nunknown = true\n",
        RustfmtTomlRequirements {
            forbidden_ignore_path_globs: ignore_globs(vec![("target/**", "no target ignore")]),
            closed_settings: Some("closed".to_owned()),
            ..RustfmtTomlRequirements::default()
        },
    );

    let expected = String::from_utf8(first_bytes(&output)).unwrap_or_default();
    assert!(
        expected.contains("ignore"),
        "ignore should be allowed when forbidden ignore glob rules manage it"
    );
    assert!(
        !expected.contains("unknown"),
        "unlisted key should still be removed"
    );
}

#[test]
fn closed_settings_remove_unlisted_keys() {
    let output = reconcile_resolved(
        "max_width = 100\nunknown = true\n",
        RustfmtTomlRequirements {
            scalar_settings: BTreeMap::from([(
                RustfmtScalarSetting::MaxWidth,
                ScalarAssertion::Equals(ConfigScalar::Int(100), "max width".to_owned()),
            )]),
            closed_settings: Some("closed".to_owned()),
            ..RustfmtTomlRequirements::default()
        },
    );

    let expected = String::from_utf8(first_bytes(&output)).unwrap_or_default();
    assert!(expected.contains("max_width"), "listed key should remain");
    assert!(
        !expected.contains("unknown"),
        "unlisted key should be removed"
    );
    assert_eq!(output.findings.len(), 1, "closed extra should report");
}

#[test]
fn closed_settings_keep_allowed_list_keys() {
    let output = reconcile_resolved(
        "ignore = [\"target\"]\nunknown = true\n",
        RustfmtTomlRequirements {
            list_settings: BTreeMap::from([(
                RustfmtListSetting::Ignore,
                ListRequirements {
                    contains: BTreeMap::from([("target".to_owned(), "ignore target".to_owned())]),
                    ..ListRequirements::default()
                },
            )]),
            closed_settings: Some("closed".to_owned()),
            ..RustfmtTomlRequirements::default()
        },
    );

    let expected = String::from_utf8(first_bytes(&output)).unwrap_or_default();
    assert!(
        expected.contains("ignore"),
        "allowed list key should remain"
    );
    assert!(
        !expected.contains("unknown"),
        "unlisted key should be removed"
    );
    assert_eq!(output.findings.len(), 1, "closed extra should report");
}

#[test]
fn unrelated_settings_are_preserved_when_not_closed() {
    let output = reconcile_resolved(
        "max_width = 80\nunknown = true\n",
        RustfmtTomlRequirements {
            scalar_settings: BTreeMap::from([(
                RustfmtScalarSetting::MaxWidth,
                ScalarAssertion::Equals(ConfigScalar::Int(100), "max width".to_owned()),
            )]),
            ..RustfmtTomlRequirements::default()
        },
    );

    let expected = String::from_utf8(first_bytes(&output)).unwrap_or_default();
    assert!(
        expected.contains("unknown = true"),
        "unrelated key should remain when settings are not closed"
    );
    assert!(
        expected.contains("max_width = 100"),
        "managed key should still be rewritten"
    );
}

fn reconcile(current: &str, req: RustfmtTomlRequirements) -> aqc_file_engine_core::EngineOutput {
    reconcile_resolved(current, req)
}

fn reconcile_resolved(
    current: &str,
    req: RustfmtTomlRequirements,
) -> aqc_file_engine_core::EngineOutput {
    let (resolved, conflicts) = RustfmtTomlRequirements::merge(vec![(
        Provenance {
            policy: "test".to_owned(),
        },
        req,
    )]);
    assert!(
        conflicts.is_empty(),
        "single requirement should not conflict: {conflicts:?}"
    );
    <RustfmtTomlEngine as FileEngine<ResolvedRustfmtTomlRequirements>>::reconcile(
        Some(current.as_bytes()),
        &resolved,
    )
}

fn ignore_globs(globs: IgnoreGlobCases<'_>) -> ForbiddenGlobRequirements<RustfmtIgnorePathGlob> {
    ForbiddenGlobRequirements {
        globs: globs
            .into_iter()
            .map(|(glob, msg)| {
                (
                    RustfmtIgnorePathGlob {
                        glob: glob.to_owned(),
                    },
                    msg.to_owned(),
                )
            })
            .collect(),
    }
}

fn first_bytes(output: &aqc_file_engine_core::EngineOutput) -> Vec<u8> {
    output
        .files
        .first()
        .map_or_else(Vec::new, |file| file.expected_bytes.clone())
}
