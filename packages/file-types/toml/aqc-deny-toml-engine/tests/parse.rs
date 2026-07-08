use aqc_toml_engine_core as _;
use serde as _;
use toml_edit as _;

use aqc_deny_toml_engine::{
    DenyLintLevel, DenyTomlEngine, DenyTomlRequirements, ResolvedDenyTomlRequirements,
    ScalarAssertion,
};
use aqc_file_engine_core::{FileEngine, Finding, Provenance};

#[test]
fn malformed_toml() {
    let output = <DenyTomlEngine as FileEngine<ResolvedDenyTomlRequirements>>::reconcile(
        Some(b"[bans\n"),
        &resolved(),
    );
    assert!(
        output
            .findings
            .iter()
            .any(|finding| matches!(finding, Finding::ParseError { .. })),
        "malformed TOML should report parse error"
    );
    assert!(
        first_bytes(&output).is_empty(),
        "parse failures should not produce expected bytes"
    );
}

#[test]
fn unknown_enum_value() {
    let output = reconcile("[bans]\nmultiple-versions = \"maybe\"\n");
    assert!(
        output
            .findings
            .iter()
            .any(|finding| matches!(finding, Finding::Mismatch { key, .. } if key == "bans.multiple-versions")),
        "unknown enum value should be reported as scalar drift"
    );
}

#[test]
fn wrong_scalar_type() {
    let output = reconcile("[bans]\nmultiple-versions = 1\n");
    assert!(
        output
            .findings
            .iter()
            .any(|finding| matches!(finding, Finding::Mismatch { key, .. } if key == "bans.multiple-versions")),
        "wrong scalar type should be reported"
    );
}

fn reconcile(current: &str) -> aqc_file_engine_core::EngineOutput {
    <DenyTomlEngine as FileEngine<ResolvedDenyTomlRequirements>>::reconcile(
        Some(current.as_bytes()),
        &resolved(),
    )
}

fn resolved() -> ResolvedDenyTomlRequirements {
    let (resolved, conflicts) = DenyTomlRequirements::merge(vec![(
        Provenance {
            policy: "test".to_owned(),
        },
        DenyTomlRequirements {
            bans_multiple_versions: Some(ScalarAssertion::Equals(
                DenyLintLevel::Deny,
                "deny duplicates".to_owned(),
            )),
            ..DenyTomlRequirements::default()
        },
    )]);
    assert!(conflicts.is_empty(), "parse fixture must merge");
    resolved
}

fn first_bytes(output: &aqc_file_engine_core::EngineOutput) -> Vec<u8> {
    output
        .files
        .first()
        .map_or_else(Vec::new, |file| file.expected_bytes.clone())
}
