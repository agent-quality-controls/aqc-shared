use aqc_toml_engine_core as _;
use std::collections::{BTreeMap, BTreeSet};

use aqc_file_engine_core::{ConfigScalar, FileEngine, Finding, Provenance, ScalarAssertion};
use aqc_rustfmt_toml_engine::{
    ResolvedRustfmtTomlRequirements, RustfmtScalarSetting, RustfmtTomlEngine,
    RustfmtTomlRequirements,
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
                ScalarAssertion::Equals(ConfigScalar::Int(100), "max width".to_owned()),
            )]),
            ..RustfmtTomlRequirements::default()
        },
    );

    let expected = String::from_utf8(first_bytes(&output)).unwrap_or_default();
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
            ScalarAssertion::Equals(ConfigScalar::Int(100), "max width".to_owned()),
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
    let expected = String::from_utf8(first_bytes(&output)).unwrap_or_default();
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
                ScalarAssertion::Absent("nightly".to_owned()),
            )]),
            ..RustfmtTomlRequirements::default()
        },
    );

    let expected = String::from_utf8(first_bytes(&output)).unwrap_or_default();
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
                ScalarAssertion::OneOf(
                    BTreeSet::from([
                        ConfigScalar::Str("2021".to_owned()),
                        ConfigScalar::Str("2024".to_owned()),
                    ]),
                    "edition set".to_owned(),
                ),
            )]),
            ..RustfmtTomlRequirements::default()
        },
    );

    let expected = String::from_utf8(first_bytes(&output)).unwrap_or_default();
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
                ScalarAssertion::Present("edition present".to_owned()),
            )]),
            ..RustfmtTomlRequirements::default()
        },
    );

    let expected = String::from_utf8(first_bytes(&output)).unwrap_or_default();
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
                ScalarAssertion::Equals(ConfigScalar::Int(100), "max width".to_owned()),
            )]),
            ..RustfmtTomlRequirements::default()
        },
    );

    let expected = String::from_utf8(first_bytes(&output)).unwrap_or_default();
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
                ScalarAssertion::Equals(ConfigScalar::Int(100), "max width".to_owned()),
            )]),
            ..RustfmtTomlRequirements::default()
        },
    );

    assert!(
        first_bytes(&output).is_empty(),
        "parse failures must not produce replacement bytes"
    );
    assert_eq!(output.findings.len(), 1, "parse failures must not cascade");
    assert!(
        matches!(output.findings.first(), Some(Finding::ParseError { .. })),
        "malformed TOML should report a parse error"
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

fn first_bytes(output: &aqc_file_engine_core::EngineOutput) -> Vec<u8> {
    output
        .files
        .first()
        .map_or_else(Vec::new, |file| file.expected_bytes.clone())
}
