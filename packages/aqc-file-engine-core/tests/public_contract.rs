use aqc_file_engine_core::{
    Engine, EngineOutput, EngineRequirement, FileEngine, Finding, Provenance,
};
use serde as _;

#[derive(Debug)]
struct ResolvedRequirements;

#[derive(Debug)]
struct DummyEngine;

impl FileEngine<ResolvedRequirements> for DummyEngine {
    fn reconcile(
        current_bytes: Option<&[u8]>,
        _resolved_requirements: &ResolvedRequirements,
    ) -> EngineOutput {
        EngineOutput {
            expected_bytes: current_bytes.map(<[u8]>::to_vec).unwrap_or_default(),
            findings: Vec::new(),
        }
    }
}

impl Engine for DummyEngine {
    fn id(&self) -> &'static str {
        "dummy"
    }

    fn reconcile(
        &self,
        current_bytes: Option<&[u8]>,
        _reqs: &[(Provenance, Box<dyn EngineRequirement>)],
    ) -> EngineOutput {
        <Self as FileEngine<ResolvedRequirements>>::reconcile(current_bytes, &ResolvedRequirements)
    }
}

#[test]
fn engine_reconcile_none_returns_single_byte_output() {
    let engine = DummyEngine;
    let output = engine.reconcile(None, &[]);
    assert_eq!(
        output.expected_bytes,
        Vec::<u8>::new(),
        "expected bytes are direct output"
    );
}

#[test]
fn finding_conflicting_requirements_has_no_subject() {
    let finding = Finding::ConflictingRequirements {
        key: "field".to_owned(),
        contributors: Vec::new(),
        reason: "scalar-disagree".to_owned(),
    };
    assert!(
        matches!(finding, Finding::ConflictingRequirements { .. }),
        "Finding::ConflictingRequirements should construct without report subject"
    );
}

#[test]
fn assert_no_path_aware_public_api() {
    let output = EngineOutput {
        expected_bytes: b"bytes".to_vec(),
        findings: Vec::new(),
    };
    assert_eq!(
        output.expected_bytes,
        b"bytes".to_vec(),
        "EngineOutput is one byte stream"
    );
}
