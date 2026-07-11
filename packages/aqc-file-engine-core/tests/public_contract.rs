use aqc_file_engine_core::{
    ConflictEntry, Engine, EngineOutput, EngineRequirement, FileEngine, Finding, Provenance,
    merged_reconcile,
};
use serde as _;

#[derive(Debug)]
struct ResolvedRequirements;

#[derive(Debug)]
struct DummyEngine;

#[derive(Debug, Clone)]
struct DummyRequirement;

impl EngineRequirement for DummyRequirement {
    fn engine_id(&self) -> &'static str {
        "dummy"
    }

    fn as_any(&self) -> &dyn core::any::Any {
        self
    }
}

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
fn merge_conflicts_preserve_current_bytes_and_skip_reconciliation() {
    let reqs: Vec<(Provenance, Box<dyn EngineRequirement>)> = vec![(
        Provenance {
            policy: "policy".to_owned(),
        },
        Box::new(DummyRequirement),
    )];
    let reconciled = core::cell::Cell::new(false);
    let output = merged_reconcile(
        Some(b"current"),
        &reqs,
        |_: Vec<(Provenance, DummyRequirement)>| {
            (
                ResolvedRequirements,
                vec![ConflictEntry {
                    key: "field".to_owned(),
                    reason: "scalar-disagree".to_owned(),
                    contributors: Vec::new(),
                }],
            )
        },
        |_, _| {
            reconciled.set(true);
            EngineOutput {
                expected_bytes: b"changed".to_vec(),
                findings: Vec::new(),
            }
        },
    );

    assert!(
        !reconciled.get(),
        "conflicted requirements must not reconcile"
    );
    assert_eq!(output.expected_bytes, b"current");
    assert_eq!(output.findings.len(), 1);
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
use schemars as _;

#[test]
fn dotted_version_schema_matches_its_transparent_string_wire_shape() {
    let schema = schemars::schema_for!(aqc_file_engine_core::DottedVersion);
    assert_eq!(
        schema.get("type").and_then(|value| value.as_str()),
        Some("string")
    );
}
