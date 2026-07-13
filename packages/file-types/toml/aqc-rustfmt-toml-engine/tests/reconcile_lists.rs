use aqc_toml_engine_core as _;
use std::collections::BTreeMap;

use aqc_file_engine_core::{FileEngine, Finding, ListRequirements, Provenance};
use aqc_rustfmt_toml_engine::{
    ResolvedRustfmtTomlRequirements, RustfmtListSetting, RustfmtTomlEngine, RustfmtTomlRequirements,
};
use globset as _;
use toml_edit as _;

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

    let expected = String::from_utf8(first_bytes(&output)).unwrap_or_default();
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

    let expected = String::from_utf8(first_bytes(&output)).unwrap_or_default();
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
    let expected = String::from_utf8(first_bytes(&output)).unwrap_or_default();
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
    let expected = String::from_utf8(first_bytes(&output)).unwrap_or_default();
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

    let expected = String::from_utf8(first_bytes(&output)).unwrap_or_default();
    assert!(
        expected.contains("ignore = []"),
        "malformed list should be normalized even when excludes does not match"
    );
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

fn first_bytes(output: &aqc_file_engine_core::EngineOutput) -> Vec<u8> {
    output.expected_bytes.clone()
}
