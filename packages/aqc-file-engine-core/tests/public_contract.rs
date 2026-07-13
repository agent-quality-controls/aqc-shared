use aqc_file_engine_core::{
    ConflictEntry, Engine, EngineOutput, EngineRequirement, FileEngine, Finding, Provenance,
    merged_reconcile,
};
use core::cell::Cell;
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
    let reconciled = Cell::new(0);
    let output = merged_reconcile(
        Some(b"current"),
        &reqs,
        |_: Vec<(Provenance, DummyRequirement)>| {
            Err(vec![ConflictEntry {
                key: "field".to_owned(),
                reason: "scalar-disagree".to_owned(),
                contributors: Vec::new(),
            }])
        },
        |_, _: &ResolvedRequirements| {
            reconciled.set(reconciled.get() + 1);
            EngineOutput {
                expected_bytes: b"changed".to_vec(),
                findings: Vec::new(),
            }
        },
    );

    assert!(
        reconciled.get() == 0,
        "conflicted requirements must not reconcile"
    );
    assert_eq!(output.expected_bytes, b"current");
    assert_eq!(output.findings.len(), 1);
}

#[test]
fn clean_merge_reconciles_exactly_once() {
    let reqs = dummy_requirements();
    let reconciled = Cell::new(0);

    let output = merged_reconcile(
        Some(b"current"),
        &reqs,
        |_: Vec<(Provenance, DummyRequirement)>| Ok(ResolvedRequirements),
        |_, _| {
            reconciled.set(reconciled.get() + 1);
            EngineOutput {
                expected_bytes: b"reconciled".to_vec(),
                findings: Vec::new(),
            }
        },
    );

    assert_eq!(reconciled.get(), 1, "clean requirements reconcile once");
    assert_eq!(output.expected_bytes, b"reconciled");
    assert!(output.findings.is_empty());
}

#[test]
fn no_matching_requirements_preserve_bytes_without_merge_or_reconcile() {
    let merged = Cell::new(false);
    let reconciled = Cell::new(false);
    let output = merged_reconcile::<DummyRequirement, ResolvedRequirements, _, _>(
        Some(b"current"),
        &[],
        |_| {
            merged.set(true);
            Ok(ResolvedRequirements)
        },
        |_, _| {
            reconciled.set(true);
            EngineOutput {
                expected_bytes: Vec::new(),
                findings: Vec::new(),
            }
        },
    );

    assert!(!merged.get(), "empty requirements must not merge");
    assert!(!reconciled.get(), "empty requirements must not reconcile");
    assert_eq!(output.expected_bytes, b"current");
    assert!(output.findings.is_empty());
}

#[test]
fn merge_conflicts_preserve_missing_bytes_as_empty() {
    let reconciled = Cell::new(false);
    let output = merged_reconcile(
        None,
        &dummy_requirements(),
        |_: Vec<(Provenance, DummyRequirement)>| {
            Err(vec![ConflictEntry {
                key: "field".to_owned(),
                reason: "scalar-disagree".to_owned(),
                contributors: Vec::new(),
            }])
        },
        |_, _: &ResolvedRequirements| {
            reconciled.set(true);
            EngineOutput {
                expected_bytes: Vec::new(),
                findings: Vec::new(),
            }
        },
    );

    assert!(
        !reconciled.get(),
        "conflicted requirements must not reconcile"
    );
    assert!(output.expected_bytes.is_empty());
    assert_eq!(output.findings.len(), 1);
}

#[test]
fn multiple_merge_conflicts_preserve_order_and_attribution() {
    let reconciled = Cell::new(false);
    let output = merged_reconcile(
        Some(b"current"),
        &dummy_requirements(),
        |_: Vec<(Provenance, DummyRequirement)>| {
            Err(vec![
                ConflictEntry {
                    key: "first".to_owned(),
                    reason: "first-reason".to_owned(),
                    contributors: vec![
                        (provenance("policy-a"), "value-a".to_owned()),
                        (provenance("policy-b"), "value-b".to_owned()),
                    ],
                },
                ConflictEntry {
                    key: "second".to_owned(),
                    reason: "second-reason".to_owned(),
                    contributors: vec![(provenance("policy-c"), "value-c".to_owned())],
                },
            ])
        },
        |_, _: &ResolvedRequirements| {
            reconciled.set(true);
            EngineOutput {
                expected_bytes: Vec::new(),
                findings: Vec::new(),
            }
        },
    );

    assert!(
        !reconciled.get(),
        "conflicted requirements must not reconcile"
    );
    assert_eq!(output.expected_bytes, b"current");
    assert!(matches!(
        output.findings.as_slice(),
        [
            Finding::ConflictingRequirements {
                key: first_key,
                reason: first_reason,
                contributors: first_contributors,
            },
            Finding::ConflictingRequirements {
                key: second_key,
                reason: second_reason,
                contributors: second_contributors,
            },
        ] if first_key == "first"
            && first_reason == "first-reason"
            && first_contributors == &vec![
                ("policy-a".to_owned(), "value-a".to_owned()),
                ("policy-b".to_owned(), "value-b".to_owned()),
            ]
            && second_key == "second"
            && second_reason == "second-reason"
            && second_contributors == &vec![("policy-c".to_owned(), "value-c".to_owned())]
    ));
}

fn dummy_requirements() -> Vec<(Provenance, Box<dyn EngineRequirement>)> {
    vec![(provenance("policy"), Box::new(DummyRequirement))]
}

fn provenance(policy: &str) -> Provenance {
    Provenance {
        policy: policy.to_owned(),
    }
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
