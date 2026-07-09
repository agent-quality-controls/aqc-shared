use aqc_toml_engine_core as _;
use serde as _;
use toml_edit as _;

use aqc_deny_toml_engine::{
    DenyLintLevel, DenyTomlEngine, DenyTomlRequirements, ResolvedDenyTomlRequirements,
    ScalarAssertion,
};
use aqc_file_engine_core::{Engine, EngineRequirement, FileEngine, Provenance};

#[test]
fn missing_file() {
    let output =
        <DenyTomlEngine as FileEngine<ResolvedDenyTomlRequirements>>::reconcile(None, &resolved());
    let expected = String::from_utf8(first_bytes(&output)).unwrap_or_default();
    assert!(
        expected.contains("multiple-versions = \"deny\""),
        "missing deny.toml should produce expected bytes"
    );
}

#[test]
fn writes_deterministic_baseline() {
    let requirement: Box<dyn EngineRequirement> = Box::new(raw());
    let output = DenyTomlEngine.reconcile(
        None,
        &[(
            Provenance {
                policy: "test".to_owned(),
            },
            requirement,
        )],
    );
    let expected = String::from_utf8(first_bytes(&output)).unwrap_or_default();
    assert!(
        expected.contains("[bans]"),
        "engine path should write deterministic section"
    );
}

#[test]
fn engine_requirement_id_matches_crate() {
    let req = raw();
    assert_eq!(req.engine_id(), aqc_deny_toml_engine::ENGINE_ID);
}

fn raw() -> DenyTomlRequirements {
    DenyTomlRequirements {
        bans_multiple_versions: Some(ScalarAssertion::Equals(
            DenyLintLevel::Deny,
            "deny duplicates".to_owned(),
        )),
        ..DenyTomlRequirements::default()
    }
}

fn resolved() -> ResolvedDenyTomlRequirements {
    let (resolved, conflicts) = DenyTomlRequirements::merge(vec![(
        Provenance {
            policy: "test".to_owned(),
        },
        raw(),
    )]);
    assert!(conflicts.is_empty(), "baseline must merge: {conflicts:?}");
    resolved
}

fn first_bytes(output: &aqc_file_engine_core::EngineOutput) -> Vec<u8> {
    output.expected_bytes.clone()
}
