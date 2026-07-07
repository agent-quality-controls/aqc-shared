use aqc_toml_engine_core as _;
use serde as _;
use toml_edit as _;

use std::collections::{BTreeMap, BTreeSet};

use aqc_deny_toml_engine::{
    DenyFeatureBanSpec, DenyGraphTargetSpec, DenyLintLevel, DenyNonEmptyString,
    DenyTomlRequirements,
};
use aqc_file_engine_core::{ItemRequirements, ListRequirements, Provenance, ScalarAssertion};

#[test]
fn conflicting_requirements_report_conflict() {
    let (_resolved, conflicts) = DenyTomlRequirements::merge(vec![
        (
            provenance("left"),
            DenyTomlRequirements {
                bans_multiple_versions: Some(ScalarAssertion::Equals(
                    DenyLintLevel::Allow,
                    "left".to_owned(),
                )),
                ..DenyTomlRequirements::default()
            },
        ),
        (
            provenance("right"),
            DenyTomlRequirements {
                bans_multiple_versions: Some(ScalarAssertion::Equals(
                    DenyLintLevel::Deny,
                    "right".to_owned(),
                )),
                ..DenyTomlRequirements::default()
            },
        ),
    ]);
    assert_eq!(conflicts.len(), 1, "scalar conflict should be reported");
}

#[test]
fn uses_core_scalar_merge() {
    let (resolved, conflicts) = DenyTomlRequirements::merge(vec![(provenance("p"), scalar_req())]);
    assert!(
        conflicts.is_empty(),
        "scalar merge should use core behavior"
    );
    assert!(resolved.bans_multiple_versions.is_some());
}

#[test]
fn uses_core_list_merge() {
    let (resolved, conflicts) = DenyTomlRequirements::merge(vec![(
        provenance("p"),
        DenyTomlRequirements {
            graph_features: ListRequirements {
                contains: BTreeMap::from([("all".to_owned(), "all features".to_owned())]),
                ..ListRequirements::default()
            },
            ..DenyTomlRequirements::default()
        },
    )]);
    assert!(conflicts.is_empty(), "list merge should use core behavior");
    assert!(resolved.graph_features.contains.contains_key("all"));
}

#[test]
fn uses_core_item_merge() {
    let target = DenyGraphTargetSpec::new("x86_64-unknown-linux-gnu")
        .expect("test target triple should construct a graph target requirement");
    let (resolved, conflicts) = DenyTomlRequirements::merge(vec![(
        provenance("p"),
        DenyTomlRequirements {
            graph_targets: ItemRequirements {
                required: vec![(target, "target".to_owned())],
                ..ItemRequirements::default()
            },
            ..DenyTomlRequirements::default()
        },
    )]);
    assert!(conflicts.is_empty(), "item merge should use core behavior");
    assert!(
        resolved
            .graph_targets
            .required
            .contains_key("x86_64-unknown-linux-gnu")
    );
}

#[test]
fn feature_allow_deny_overlap_is_invalid() {
    let feature =
        DenyNonEmptyString::new("std").expect("test feature name should construct a deny string");
    let err = DenyFeatureBanSpec::new(
        "serde",
        BTreeSet::from([feature.clone()]),
        BTreeSet::from([feature]),
    )
    .expect_err("overlap must be invalid");
    assert!(
        err.to_string().contains("serde"),
        "error should identify package"
    );
}

#[test]
fn list_order_is_ignored() {
    let (resolved, conflicts) = DenyTomlRequirements::merge(vec![(provenance("p"), exact_list())]);
    assert!(
        conflicts.is_empty(),
        "single exact list should not conflict"
    );
    assert_eq!(
        resolved.graph_features.exact.expect("exact").merged,
        vec!["a".to_owned(), "b".to_owned()]
    );
}

fn scalar_req() -> DenyTomlRequirements {
    DenyTomlRequirements {
        bans_multiple_versions: Some(ScalarAssertion::Equals(
            DenyLintLevel::Deny,
            "deny".to_owned(),
        )),
        ..DenyTomlRequirements::default()
    }
}

fn exact_list() -> DenyTomlRequirements {
    DenyTomlRequirements {
        graph_features: ListRequirements {
            exact: Some((vec!["b".to_owned(), "a".to_owned()], "exact".to_owned())),
            ..ListRequirements::default()
        },
        ..DenyTomlRequirements::default()
    }
}

fn provenance(policy: &str) -> Provenance {
    Provenance {
        policy: policy.to_owned(),
    }
}
