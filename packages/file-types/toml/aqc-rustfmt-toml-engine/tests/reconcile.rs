use std::collections::{BTreeMap, BTreeSet};

use aqc_file_engine_core::{
    ConfigScalar, FileEngine, Finding, ForbiddenGlobRequirements, ListRequirements, Provenance,
};
use aqc_rustfmt_toml_engine::{
    ResolvedRustfmtTomlRequirements, RustfmtIgnorePathGlob, RustfmtListSetting,
    RustfmtScalarAssertion, RustfmtScalarSetting, RustfmtTomlEngine, RustfmtTomlRequirements,
};
use globset as _;
use toml_edit as _;

#[test]
fn equals_writes_missing_scalar() {
    let output = reconcile(
        "",
        RustfmtTomlRequirements {
            scalar_settings: BTreeMap::from([(
                RustfmtScalarSetting::MaxWidth,
                RustfmtScalarAssertion::Equals(ConfigScalar::Int(100), "max width".to_owned()),
            )]),
            ..RustfmtTomlRequirements::default()
        },
    );

    let expected = String::from_utf8(output.expected_bytes).unwrap_or_default();
    assert!(
        expected.contains("max_width = 100"),
        "init bytes should write max_width"
    );
    assert_eq!(
        output.findings.len(),
        1,
        "missing scalar should be reported"
    );
}

#[test]
fn missing_file_writes_scalar() {
    let req = RustfmtTomlRequirements {
        scalar_settings: BTreeMap::from([(
            RustfmtScalarSetting::MaxWidth,
            RustfmtScalarAssertion::Equals(ConfigScalar::Int(100), "max width".to_owned()),
        )]),
        ..RustfmtTomlRequirements::default()
    };
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

    let output = <RustfmtTomlEngine as FileEngine<ResolvedRustfmtTomlRequirements>>::reconcile(
        None, &resolved,
    );
    let expected = String::from_utf8(output.expected_bytes).unwrap_or_default();
    assert!(
        expected.contains("max_width = 100"),
        "missing file should be initialized"
    );
    assert_eq!(
        output.findings.len(),
        1,
        "missing file write should report the missing scalar"
    );
}

#[test]
fn absent_removes_existing_scalar() {
    let output = reconcile(
        "group_imports = \"One\"\n",
        RustfmtTomlRequirements {
            scalar_settings: BTreeMap::from([(
                RustfmtScalarSetting::GroupImports,
                RustfmtScalarAssertion::Absent("nightly".to_owned()),
            )]),
            ..RustfmtTomlRequirements::default()
        },
    );

    let expected = String::from_utf8(output.expected_bytes).unwrap_or_default();
    assert!(
        !expected.contains("group_imports"),
        "absent scalar should be removed"
    );
    assert_eq!(
        output.findings.len(),
        1,
        "existing forbidden scalar should report"
    );
}

#[test]
fn one_of_reports_without_writing_choice() {
    let output = reconcile(
        "",
        RustfmtTomlRequirements {
            scalar_settings: BTreeMap::from([(
                RustfmtScalarSetting::Edition,
                RustfmtScalarAssertion::OneOf(
                    BTreeSet::from(["2021".to_owned(), "2024".to_owned()]),
                    "edition set".to_owned(),
                ),
            )]),
            ..RustfmtTomlRequirements::default()
        },
    );

    let expected = String::from_utf8(output.expected_bytes).unwrap_or_default();
    assert!(
        !expected.contains("edition"),
        "check-only one-of should not write a value"
    );
    assert_eq!(output.findings.len(), 1, "missing one-of should report");
}

#[test]
fn present_reports_missing_without_writing_choice() {
    let output = reconcile(
        "",
        RustfmtTomlRequirements {
            scalar_settings: BTreeMap::from([(
                RustfmtScalarSetting::Edition,
                RustfmtScalarAssertion::Present("edition present".to_owned()),
            )]),
            ..RustfmtTomlRequirements::default()
        },
    );

    let expected = String::from_utf8(output.expected_bytes).unwrap_or_default();
    assert!(
        !expected.contains("edition"),
        "present check should not write a value"
    );
    assert_eq!(output.findings.len(), 1, "missing present should report");
}

#[test]
fn wrong_scalar_type_writes_desired_value() {
    let output = reconcile(
        "max_width = \"wide\"\n",
        RustfmtTomlRequirements {
            scalar_settings: BTreeMap::from([(
                RustfmtScalarSetting::MaxWidth,
                RustfmtScalarAssertion::Equals(ConfigScalar::Int(100), "max width".to_owned()),
            )]),
            ..RustfmtTomlRequirements::default()
        },
    );

    let expected = String::from_utf8(output.expected_bytes).unwrap_or_default();
    assert!(
        expected.contains("max_width = 100"),
        "wrong scalar type should be replaced"
    );
    assert_eq!(output.findings.len(), 1, "wrong scalar type should report");
}

#[test]
fn malformed_toml_reports_parse_error() {
    let output = reconcile(
        "max_width = ",
        RustfmtTomlRequirements {
            scalar_settings: BTreeMap::from([(
                RustfmtScalarSetting::MaxWidth,
                RustfmtScalarAssertion::Equals(ConfigScalar::Int(100), "max width".to_owned()),
            )]),
            ..RustfmtTomlRequirements::default()
        },
    );

    assert!(
        matches!(output.findings.first(), Some(Finding::ParseError { .. })),
        "malformed TOML should report a parse error"
    );
}

#[test]
fn list_contains_and_excludes_reconcile_values() {
    let output = reconcile(
        "ignore = [\"old\", \"kept\"]\n",
        RustfmtTomlRequirements {
            list_settings: BTreeMap::from([(
                RustfmtListSetting::Ignore,
                ListRequirements {
                    contains: BTreeMap::from([("new".to_owned(), "include new".to_owned())]),
                    excludes: BTreeMap::from([("old".to_owned(), "exclude old".to_owned())]),
                    exact: None,
                },
            )]),
            ..RustfmtTomlRequirements::default()
        },
    );

    let expected = String::from_utf8(output.expected_bytes).unwrap_or_default();
    assert!(expected.contains("\"kept\""), "existing item should remain");
    assert!(
        expected.contains("\"new\""),
        "contains item should be added"
    );
    assert!(
        !expected.contains("\"old\""),
        "excluded item should be removed"
    );
    assert_eq!(
        output.findings.len(),
        2,
        "one add and one remove should report"
    );
}

#[test]
fn exact_list_writes_exact_values() {
    let output = reconcile(
        "ignore = [\"old\"]\n",
        RustfmtTomlRequirements {
            list_settings: BTreeMap::from([(
                RustfmtListSetting::Ignore,
                ListRequirements {
                    exact: Some((vec!["generated".to_owned()], "exact ignore".to_owned())),
                    ..ListRequirements::default()
                },
            )]),
            ..RustfmtTomlRequirements::default()
        },
    );

    let expected = String::from_utf8(output.expected_bytes).unwrap_or_default();
    assert!(
        expected.contains("\"generated\""),
        "exact list should write required value"
    );
    assert!(
        !expected.contains("\"old\""),
        "exact list should remove old value"
    );
    assert_eq!(
        output.findings.len(),
        1,
        "exact list mismatch should report"
    );
}

#[test]
fn non_array_list_setting_reports_shape_finding() {
    let output = reconcile(
        "ignore = \"target\"\n",
        RustfmtTomlRequirements {
            list_settings: BTreeMap::from([(
                RustfmtListSetting::Ignore,
                ListRequirements {
                    contains: BTreeMap::from([("target".to_owned(), "ignore target".to_owned())]),
                    ..ListRequirements::default()
                },
            )]),
            ..RustfmtTomlRequirements::default()
        },
    );

    assert!(
        output.findings.iter().any(
            |finding| matches!(finding, Finding::Mismatch { key, expected, .. } if key == "ignore" && expected == "array of strings")
        ),
        "non-array list setting should report shape finding"
    );
    let expected = String::from_utf8(output.expected_bytes).unwrap_or_default();
    assert!(
        expected.contains("ignore = [\"target\"]"),
        "non-array list setting should be rewritten to a string array"
    );
}

#[test]
fn non_string_list_item_reports_shape_finding() {
    let output = reconcile(
        "ignore = [\"target\", 42]\n",
        RustfmtTomlRequirements {
            list_settings: BTreeMap::from([(
                RustfmtListSetting::Ignore,
                ListRequirements {
                    contains: BTreeMap::from([(
                        "generated".to_owned(),
                        "ignore generated".to_owned(),
                    )]),
                    ..ListRequirements::default()
                },
            )]),
            ..RustfmtTomlRequirements::default()
        },
    );

    assert!(
        output.findings.iter().any(
            |finding| matches!(finding, Finding::Mismatch { key, expected, .. } if key == "ignore[1]" && expected == "string")
        ),
        "non-string list item should report shape finding"
    );
    let expected = String::from_utf8(output.expected_bytes).unwrap_or_default();
    assert!(
        expected.contains("ignore = [\"target\", \"generated\"]"),
        "non-string list item should be removed before writing"
    );
}

#[test]
fn malformed_list_is_normalized_even_without_matching_item_change() {
    let output = reconcile(
        "ignore = [42]\n",
        RustfmtTomlRequirements {
            list_settings: BTreeMap::from([(
                RustfmtListSetting::Ignore,
                ListRequirements {
                    excludes: BTreeMap::from([(
                        "unmatched".to_owned(),
                        "exclude unmatched".to_owned(),
                    )]),
                    ..ListRequirements::default()
                },
            )]),
            ..RustfmtTomlRequirements::default()
        },
    );

    let expected = String::from_utf8(output.expected_bytes).unwrap_or_default();
    assert!(
        expected.contains("ignore = []"),
        "malformed list should be normalized even when excludes does not match"
    );
}

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

    let expected = String::from_utf8(output.expected_bytes).unwrap_or_default();
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
            |finding| matches!(finding, Finding::Mismatch { key, expected, .. } if key == "ignore.target/generated" && expected == "absent (path glob)")
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

    let expected = String::from_utf8(output.expected_bytes).unwrap_or_default();
    assert!(
        expected.contains("ignore = [\"target/generated\"]"),
        "malformed ignore list should be normalized when glob rules inspect it"
    );
    assert!(
        output.findings.iter().any(
            |finding| matches!(finding, Finding::Mismatch { key, expected, .. } if key == "ignore[1]" && expected == "string")
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

    let expected = String::from_utf8(output.expected_bytes).unwrap_or_default();
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
                RustfmtScalarAssertion::Equals(ConfigScalar::Int(100), "max width".to_owned()),
            )]),
            closed_settings: Some("closed".to_owned()),
            ..RustfmtTomlRequirements::default()
        },
    );

    let expected = String::from_utf8(output.expected_bytes).unwrap_or_default();
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

    let expected = String::from_utf8(output.expected_bytes).unwrap_or_default();
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
                RustfmtScalarAssertion::Equals(ConfigScalar::Int(100), "max width".to_owned()),
            )]),
            ..RustfmtTomlRequirements::default()
        },
    );

    let expected = String::from_utf8(output.expected_bytes).unwrap_or_default();
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

fn ignore_globs(globs: Vec<(&str, &str)>) -> ForbiddenGlobRequirements<RustfmtIgnorePathGlob> {
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
