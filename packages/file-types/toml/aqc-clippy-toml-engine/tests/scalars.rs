use aqc_toml_engine_core as _;
use std::collections::{BTreeMap, BTreeSet};

use aqc_clippy_toml_engine::{ClippyTomlEngine, ClippyTomlRequirements};
use aqc_file_engine_core::{
    DottedVersion, Engine, EngineRequirement, Finding, Provenance, ScalarAssertion,
};
use globset as _;
use toml_edit as _;

type ClippyRequirementInput = (Provenance, ClippyTomlRequirements);

fn prov(policy: &str) -> Provenance {
    Provenance {
        policy: policy.to_owned(),
    }
}

fn clippy_findings(reqs: Vec<ClippyRequirementInput>) -> Vec<Finding> {
    clippy_output(Some(b""), reqs).findings
}

fn clippy_output(
    bytes: Option<&[u8]>,
    reqs: Vec<ClippyRequirementInput>,
) -> aqc_file_engine_core::EngineOutput {
    let reqs = reqs
        .into_iter()
        .map(|(p, r)| {
            let requirement: Box<dyn EngineRequirement> = Box::new(r);
            (p, requirement)
        })
        .collect::<Vec<_>>();
    ClippyTomlEngine.reconcile(bytes, &reqs)
}

#[test]
fn clippy_thresholds_compose_per_key() {
    let left = ClippyTomlRequirements {
        thresholds: BTreeMap::from([(
            "too-many-lines-threshold".to_owned(),
            ScalarAssertion::AtMost(100, "limit".to_owned()),
        )]),
        ..ClippyTomlRequirements::default()
    };
    let right = ClippyTomlRequirements {
        thresholds: BTreeMap::from([(
            "too-many-lines-threshold".to_owned(),
            ScalarAssertion::AtMost(80, "stricter".to_owned()),
        )]),
        ..ClippyTomlRequirements::default()
    };
    let (merged, conflicts) =
        ClippyTomlRequirements::merge(vec![(prov("p1"), left), (prov("p2"), right)]);
    assert!(conflicts.is_empty());
    let threshold = merged
        .thresholds
        .get("too-many-lines-threshold")
        .expect("merged clippy requirements should contain too-many-lines-threshold");
    assert!(matches!(threshold.merged, ScalarAssertion::AtMost(80, _)));
}

#[test]
fn clippy_threshold_range_bounds_compose() {
    let left = ClippyTomlRequirements {
        thresholds: BTreeMap::from([(
            "too-many-lines-threshold".to_owned(),
            ScalarAssertion::AtLeast(40, "floor".to_owned()),
        )]),
        ..ClippyTomlRequirements::default()
    };
    let right = ClippyTomlRequirements {
        thresholds: BTreeMap::from([(
            "too-many-lines-threshold".to_owned(),
            ScalarAssertion::AtMost(80, "ceiling".to_owned()),
        )]),
        ..ClippyTomlRequirements::default()
    };
    let (merged, conflicts) =
        ClippyTomlRequirements::merge(vec![(prov("p1"), left), (prov("p2"), right)]);
    assert!(conflicts.is_empty());
    let threshold = merged
        .thresholds
        .get("too-many-lines-threshold")
        .expect("merged clippy requirements should contain too-many-lines-threshold");
    assert!(matches!(
        threshold.merged,
        ScalarAssertion::Range(40, 80, _)
    ));
}

#[test]
fn clippy_msrv_keeps_strongest_floor() {
    let left = ClippyTomlRequirements {
        msrv: Some(ScalarAssertion::AtLeast(
            DottedVersion::new("1.80"),
            "old".to_owned(),
        )),
        ..ClippyTomlRequirements::default()
    };
    let right = ClippyTomlRequirements {
        msrv: Some(ScalarAssertion::AtLeast(
            DottedVersion::new("1.85"),
            "new".to_owned(),
        )),
        ..ClippyTomlRequirements::default()
    };
    let (merged, conflicts) =
        ClippyTomlRequirements::merge(vec![(prov("p1"), left), (prov("p2"), right)]);
    let msrv = merged
        .msrv
        .expect("merged clippy requirements should contain an msrv assertion")
        .merged;
    assert!(conflicts.is_empty());
    assert!(matches!(
        msrv,
        ScalarAssertion::AtLeast(ref version, _) if version.as_str() == "1.85"
    ));
}

#[test]
fn clippy_requirements_use_core_scalar_assertions() {
    let req = ClippyTomlRequirements {
        msrv: Some(ScalarAssertion::AtLeast(
            DottedVersion::new("1.85"),
            "msrv".to_owned(),
        )),
        thresholds: BTreeMap::from([(
            "too-many-lines-threshold".to_owned(),
            ScalarAssertion::AtMost(75, "limit".to_owned()),
        )]),
        bools: BTreeMap::from([(
            "allow-dbg-in-tests".to_owned(),
            ScalarAssertion::Equals(false, "bool".to_owned()),
        )]),
        enums: BTreeMap::from([(
            "mode".to_owned(),
            ScalarAssertion::Equals("deny".to_owned(), "enum".to_owned()),
        )]),
        ..ClippyTomlRequirements::default()
    };
    let (merged, conflicts) = ClippyTomlRequirements::merge(vec![(prov("p1"), req)]);

    assert!(conflicts.is_empty());
    assert!(merged.msrv.is_some());
    assert!(merged.thresholds.contains_key("too-many-lines-threshold"));
    assert!(merged.bools.contains_key("allow-dbg-in-tests"));
    assert!(merged.enums.contains_key("mode"));
}

#[test]
fn clippy_rejects_scalar_operations_outside_field_family() {
    let req = ClippyTomlRequirements {
        msrv: Some(ScalarAssertion::Range(
            DottedVersion::new("1.80"),
            DottedVersion::new("1.90"),
            "range".to_owned(),
        )),
        thresholds: BTreeMap::from([(
            "too-many-lines-threshold".to_owned(),
            ScalarAssertion::OneOf(BTreeSet::from([75]), "oneof".to_owned()),
        )]),
        bools: BTreeMap::from([(
            "allow-dbg-in-tests".to_owned(),
            ScalarAssertion::OneOf(BTreeSet::from([false]), "oneof".to_owned()),
        )]),
        ..ClippyTomlRequirements::default()
    };
    let (merged, conflicts) = ClippyTomlRequirements::merge(vec![(prov("p1"), req)]);

    assert!(merged.msrv.is_none());
    assert!(merged.thresholds.is_empty());
    assert!(merged.bools.is_empty());
    assert_eq!(
        conflicts
            .iter()
            .filter(|conflict| conflict.reason == "scalar-operation-unsupported")
            .count(),
        3
    );
}

#[test]
fn clippy_scalar_family_validation_covers_all_families() {
    let req = ClippyTomlRequirements {
        msrv: Some(ScalarAssertion::AtMost(
            DottedVersion::new("1.85"),
            "msrv ceiling".to_owned(),
        )),
        thresholds: BTreeMap::from([(
            "too-many-lines-threshold".to_owned(),
            ScalarAssertion::Present("threshold exists".to_owned()),
        )]),
        bools: BTreeMap::from([
            (
                "avoid-breaking-exported-api".to_owned(),
                ScalarAssertion::AtLeast(true, "ordered bool".to_owned()),
            ),
            (
                "allow-mixed-uninlined-format-args".to_owned(),
                ScalarAssertion::Absent("bool absent".to_owned()),
            ),
        ]),
        enums: BTreeMap::from([
            (
                "msrv-policy".to_owned(),
                ScalarAssertion::AtMost("stable".to_owned(), "ordered enum".to_owned()),
            ),
            (
                "enum-present".to_owned(),
                ScalarAssertion::Present("enum present".to_owned()),
            ),
            (
                "enum-absent".to_owned(),
                ScalarAssertion::Absent("enum absent".to_owned()),
            ),
        ]),
        ..ClippyTomlRequirements::default()
    };

    let (merged, conflicts) = ClippyTomlRequirements::merge(vec![(prov("p1"), req)]);
    assert!(merged.msrv.is_none());
    assert!(merged.thresholds.contains_key("too-many-lines-threshold"));
    assert!(
        merged
            .bools
            .contains_key("allow-mixed-uninlined-format-args")
    );
    assert!(merged.enums.contains_key("enum-present"));
    assert!(merged.enums.contains_key("enum-absent"));
    assert_eq!(
        conflicts
            .iter()
            .filter(|conflict| conflict.reason == "scalar-operation-unsupported")
            .count(),
        3
    );
}

#[test]
fn clippy_scalar_implication_cases_compose() {
    let mut allowed = BTreeSet::new();
    let _ = allowed.insert("warn".to_owned());
    let left = ClippyTomlRequirements {
        enums: BTreeMap::from([(
            "disallowed-names".to_owned(),
            ScalarAssertion::Equals("warn".to_owned(), "equals".to_owned()),
        )]),
        ..ClippyTomlRequirements::default()
    };
    let right = ClippyTomlRequirements {
        enums: BTreeMap::from([(
            "disallowed-names".to_owned(),
            ScalarAssertion::OneOf(allowed, "one".to_owned()),
        )]),
        ..ClippyTomlRequirements::default()
    };
    let mut third = ClippyTomlRequirements::default();
    let _ = third.enums.insert(
        "disallowed-names".to_owned(),
        ScalarAssertion::Present("present".to_owned()),
    );
    let (merged, conflicts) = ClippyTomlRequirements::merge(vec![
        (prov("p1"), left),
        (prov("p2"), right),
        (prov("p3"), third),
    ]);
    assert!(conflicts.is_empty());
    let names = merged
        .enums
        .get("disallowed-names")
        .expect("merged clippy requirements should contain disallowed-names");
    assert!(matches!(
        names.merged,
        ScalarAssertion::Equals(ref value, _) if value == "warn"
    ));
}

#[test]
fn clippy_scalar_implication_attributes_only_failed_assertions() {
    let mut allowed = BTreeSet::new();
    let _ = allowed.insert("warn".to_owned());
    let _ = allowed.insert("deny".to_owned());
    let equals_policy = ClippyTomlRequirements {
        enums: BTreeMap::from([(
            "mode".to_owned(),
            ScalarAssertion::Equals("deny".to_owned(), "equals".to_owned()),
        )]),
        ..ClippyTomlRequirements::default()
    };
    let oneof_policy = ClippyTomlRequirements {
        enums: BTreeMap::from([(
            "mode".to_owned(),
            ScalarAssertion::OneOf(allowed, "one".to_owned()),
        )]),
        ..ClippyTomlRequirements::default()
    };
    let present_policy = ClippyTomlRequirements {
        enums: BTreeMap::from([(
            "mode".to_owned(),
            ScalarAssertion::Present("present".to_owned()),
        )]),
        ..ClippyTomlRequirements::default()
    };
    let output = clippy_output(
        Some(
            br#"mode = "warn"
"#,
        ),
        vec![
            (prov("equals-policy"), equals_policy),
            (prov("oneof-policy"), oneof_policy),
            (prov("present-policy"), present_policy),
        ],
    );
    let mismatch = output
        .findings
        .iter()
        .find(|finding| matches!(finding, Finding::Mismatch { message, .. } if message == "equals"))
        .expect("expected an equals mismatch finding");
    assert!(matches!(
        mismatch,
        Finding::Mismatch { attribution, .. }
            if attribution.iter().any(|p| p.policy == "equals-policy")
                && attribution.iter().all(|p| p.policy != "oneof-policy" && p.policy != "present-policy")
    ));
}

#[test]
fn clippy_scalar_incompatible_cases_conflict() {
    let left = ClippyTomlRequirements {
        bools: BTreeMap::from([(
            "allow-dbg-in-tests".to_owned(),
            ScalarAssertion::Equals(true, "yes".to_owned()),
        )]),
        ..ClippyTomlRequirements::default()
    };
    let right = ClippyTomlRequirements {
        bools: BTreeMap::from([(
            "allow-dbg-in-tests".to_owned(),
            ScalarAssertion::Equals(false, "no".to_owned()),
        )]),
        ..ClippyTomlRequirements::default()
    };
    let findings = clippy_findings(vec![(prov("p1"), left), (prov("p2"), right)]);
    assert!(
        findings
            .iter()
            .any(|f| matches!(f, Finding::ConflictingRequirements { .. }))
    );
}
