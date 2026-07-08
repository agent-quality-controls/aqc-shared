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
    let output = DenyTomlEngine.reconcile(
        std::path::Path::new("/tmp/work"),
        &[],
        &[(
            Provenance {
                policy: "test".to_owned(),
            },
            Box::new(raw()) as Box<dyn EngineRequirement>,
        )],
    );
    let expected = String::from_utf8(first_bytes(&output)).unwrap_or_default();
    assert!(
        expected.contains("[bans]"),
        "engine path should write deterministic section"
    );
}

#[test]
fn engine_requirement_id_and_path() {
    let req = raw();
    assert_eq!(req.engine_id(), aqc_deny_toml_engine::ENGINE_ID);
    assert_eq!(
        DenyTomlEngine
            .target_paths(std::path::Path::new("/tmp/work"), &[])
            .first()
            .map_or_else(std::path::PathBuf::new, Clone::clone)
            .ends_with("deny.toml"),
        true,
        "target path must be deny.toml"
    );
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
    output
        .files
        .first()
        .map_or_else(Vec::new, |file| file.expected_bytes.clone())
}
