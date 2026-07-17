use aqc_toml_engine_core as _;
use std::collections::BTreeMap;

use aqc_file_engine_core::{
    ConfigScalar, FileEngine, Finding, ForbiddenGlobRequirements, ItemRequirements, KeyedItem,
    ListRequirements, Provenance, ScalarAssertion,
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
fn explicit_membership_keeps_ignore_when_only_glob_rules_manage_it() {
    let output = reconcile_resolved(
        "ignore = [\"src/lib.rs\"]\nunknown = true\n",
        RustfmtTomlRequirements {
            forbidden_ignore_path_globs: ignore_globs(vec![("target/**", "no target ignore")]),
            setting_keys: exact_keys(["ignore"], "exact"),
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
fn explicit_membership_removes_unlisted_keys() {
    let output = reconcile_resolved(
        "max_width = 100\nunknown = true\n",
        RustfmtTomlRequirements {
            scalar_settings: BTreeMap::from([(
                RustfmtScalarSetting::MaxWidth,
                ScalarAssertion::Equals(ConfigScalar::Int(100), "max width".to_owned()),
            )]),
            setting_keys: exact_keys(["max_width"], "exact"),
            ..RustfmtTomlRequirements::default()
        },
    );

    let expected = String::from_utf8(first_bytes(&output)).unwrap_or_default();
    assert!(expected.contains("max_width"), "listed key should remain");
    assert!(
        !expected.contains("unknown"),
        "unlisted key should be removed"
    );
    assert_eq!(output.findings.len(), 1, "exact extra should report");
}

#[test]
fn explicit_membership_keeps_allowed_list_keys() {
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
            setting_keys: exact_keys(["ignore"], "exact"),
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
    assert_eq!(output.findings.len(), 1, "exact extra should report");
}

#[test]
fn unrelated_settings_are_preserved_when_not_exact() {
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
        "unrelated key should remain when settings are not exact"
    );
    assert!(
        expected.contains("max_width = 100"),
        "managed key should still be rewritten"
    );
}

#[test]
fn explicit_setting_membership_reports_missing_and_forbidden_keys() {
    let output = reconcile_resolved(
        "forbidden = true\n",
        RustfmtTomlRequirements {
            setting_keys: ItemRequirements {
                required: vec![(setting_key("missing"), "required key".to_owned())],
                forbidden: vec![(setting_key("forbidden"), "forbidden key".to_owned())],
                ..ItemRequirements::default()
            },
            ..RustfmtTomlRequirements::default()
        },
    );

    assert!(output.findings.iter().any(|finding| matches!(
        finding,
        Finding::UnwritableRequiredKey { key, .. } if key == "missing"
    )));
    assert!(output.findings.iter().any(|finding| matches!(
        finding,
        Finding::Mismatch { key, message, .. }
            if key == "forbidden" && message == "forbidden key"
    )));
    assert!(
        !String::from_utf8(first_bytes(&output))
            .unwrap_or_default()
            .contains("forbidden")
    );
}

#[test]
fn absent_scalar_is_excluded_from_exact_membership() {
    let output = reconcile_resolved(
        "max_width = 100\n",
        RustfmtTomlRequirements {
            scalar_settings: BTreeMap::from([(
                RustfmtScalarSetting::MaxWidth,
                ScalarAssertion::Absent("max width must be absent".to_owned()),
            )]),
            setting_keys: exact_keys([], "no present settings"),
            ..RustfmtTomlRequirements::default()
        },
    );

    assert_eq!(output.findings.len(), 1);
    assert!(
        !String::from_utf8(first_bytes(&output))
            .unwrap_or_default()
            .contains("max_width")
    );
}

#[test]
fn constructive_setting_membership_initializes_to_a_fixed_point() {
    let requirement = RustfmtTomlRequirements {
        scalar_settings: BTreeMap::from([(
            RustfmtScalarSetting::MaxWidth,
            ScalarAssertion::Equals(ConfigScalar::Int(100), "max width".to_owned()),
        )]),
        setting_keys: exact_keys(["max_width"], "only max width"),
        ..RustfmtTomlRequirements::default()
    };
    let resolved = RustfmtTomlRequirements::merge(vec![(
        Provenance {
            policy: "policy".to_owned(),
        },
        requirement,
    )])
    .expect("requirements must merge");
    let initialized = <RustfmtTomlEngine as FileEngine<_>>::reconcile(None, &resolved);
    let second = <RustfmtTomlEngine as FileEngine<_>>::reconcile(
        Some(&initialized.expected_bytes),
        &resolved,
    );

    assert!(
        String::from_utf8(initialized.expected_bytes)
            .unwrap_or_default()
            .contains("max_width = 100")
    );
    assert!(second.findings.is_empty());
}

#[test]
fn conflicting_exact_setting_keys_fail_merge() {
    let requirement = |key_name: &str| RustfmtTomlRequirements {
        setting_keys: exact_keys([key_name], key_name),
        ..RustfmtTomlRequirements::default()
    };
    let conflicts = RustfmtTomlRequirements::merge(vec![
        (
            Provenance {
                policy: "one".to_owned(),
            },
            requirement("max_width"),
        ),
        (
            Provenance {
                policy: "two".to_owned(),
            },
            requirement("hard_tabs"),
        ),
    ])
    .expect_err("different exact setting keys must conflict");

    assert!(
        conflicts
            .iter()
            .any(|conflict| conflict.key == "rustfmt.toml")
    );
}

#[test]
fn exact_setting_keys_cannot_exclude_a_constructive_value_requirement() {
    let requirement = RustfmtTomlRequirements {
        scalar_settings: BTreeMap::from([(
            RustfmtScalarSetting::MaxWidth,
            ScalarAssertion::Equals(ConfigScalar::Int(100), "max width".to_owned()),
        )]),
        setting_keys: exact_keys([], "no settings"),
        ..RustfmtTomlRequirements::default()
    };

    let conflicts = RustfmtTomlRequirements::merge(vec![(
        Provenance {
            policy: "policy".to_owned(),
        },
        requirement,
    )])
    .expect_err("value and membership requirements must conflict");

    assert!(
        conflicts
            .iter()
            .any(|conflict| conflict.key == "rustfmt.toml.max_width")
    );
}

#[test]
fn setting_membership_findings_include_all_exact_contributors() {
    let requirement = || RustfmtTomlRequirements {
        setting_keys: exact_keys([], "no settings"),
        ..RustfmtTomlRequirements::default()
    };
    let resolved = RustfmtTomlRequirements::merge(vec![
        (
            Provenance {
                policy: "two".to_owned(),
            },
            requirement(),
        ),
        (
            Provenance {
                policy: "one".to_owned(),
            },
            requirement(),
        ),
    ])
    .expect("agreeing requirements must merge");
    let output =
        <RustfmtTomlEngine as FileEngine<_>>::reconcile(Some(b"unknown = true\n"), &resolved);

    assert!(matches!(
        output.findings.as_slice(),
        [Finding::Mismatch { attribution, .. }]
            if attribution == &[
                Provenance { policy: "one".to_owned() },
                Provenance { policy: "two".to_owned() },
            ]
    ));
}

fn setting_key(file_key: &str) -> KeyedItem<()> {
    KeyedItem {
        file_key: file_key.to_owned(),
        value: (),
    }
}

fn exact_keys<const N: usize>(keys: [&str; N], message: &str) -> ItemRequirements<KeyedItem<()>> {
    ItemRequirements {
        allowed: None,
        exact: Some((
            keys.into_iter().map(setting_key).collect(),
            message.to_owned(),
        )),
        ..ItemRequirements::default()
    }
}

fn reconcile(current: &str, req: RustfmtTomlRequirements) -> aqc_file_engine_core::EngineOutput {
    reconcile_resolved(current, req)
}

fn reconcile_resolved(
    current: &str,
    req: RustfmtTomlRequirements,
) -> aqc_file_engine_core::EngineOutput {
    let result = RustfmtTomlRequirements::merge(vec![(
        Provenance {
            policy: "test".to_owned(),
        },
        req,
    )]);
    assert!(result.is_ok(), "single requirement should not conflict");
    let resolved = result.unwrap_or_default();
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
    output.expected_bytes.clone()
}
