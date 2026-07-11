#![allow(
    clippy::expect_used,
    reason = "These tests use expect messages as assertion failure labels."
)]

use std::collections::BTreeSet;

use aqc_file_engine_core::{
    DottedVersion, Provenance, Resolve, ScalarAssertion, merge::Contributor, push_conflict,
    resolve_exact_list,
};
use serde as _;

fn prov(policy: &str) -> Provenance {
    Provenance {
        policy: policy.to_owned(),
    }
}

#[test]
fn scalar_assertion_equals_and_oneof_compose() {
    let mut conflicts = Vec::new();
    let resolved = ScalarAssertion::resolve(
        "edition",
        vec![
            (
                prov("equals"),
                ScalarAssertion::Equals("2024".to_owned(), "equals".to_owned()),
            ),
            (
                prov("oneof"),
                ScalarAssertion::OneOf(BTreeSet::from(["2024".to_owned()]), "oneof".to_owned()),
            ),
        ],
        &mut conflicts,
    )
    .expect("compatible scalar assertions should resolve");

    assert!(conflicts.is_empty());
    assert!(matches!(
        resolved.merged,
        ScalarAssertion::Equals(ref value, _) if value == "2024"
    ));
}

#[test]
fn scalar_assertion_absent_conflicts_with_present() {
    let mut conflicts = Vec::new();
    let resolved = ScalarAssertion::<String>::resolve(
        "edition",
        vec![
            (
                prov("present"),
                ScalarAssertion::Present("present".to_owned()),
            ),
            (prov("absent"), ScalarAssertion::Absent("absent".to_owned())),
        ],
        &mut conflicts,
    );

    assert!(resolved.is_none());
    assert_eq!(conflicts.len(), 1);
    assert_eq!(
        conflicts.first().expect("one conflict").reason,
        "scalar-disagree"
    );
    assert_eq!(
        conflicts.first().expect("one conflict").contributors,
        vec![
            (prov("present"), "present".to_owned()),
            (prov("absent"), "absent".to_owned())
        ]
    );
}

#[test]
fn scalar_assertion_absent_and_present_merge_with_same_operation() {
    let mut conflicts = Vec::new();
    let absent = ScalarAssertion::<String>::resolve(
        "edition",
        vec![
            (prov("left"), ScalarAssertion::Absent("left".to_owned())),
            (prov("right"), ScalarAssertion::Absent("right".to_owned())),
        ],
        &mut conflicts,
    )
    .expect("same absent assertions should resolve");

    assert!(conflicts.is_empty());
    assert!(matches!(absent.merged, ScalarAssertion::Absent(_)));

    let present = ScalarAssertion::<String>::resolve(
        "edition",
        vec![
            (prov("left"), ScalarAssertion::Present("left".to_owned())),
            (prov("right"), ScalarAssertion::Present("right".to_owned())),
        ],
        &mut conflicts,
    )
    .expect("same present assertions should resolve");

    assert!(conflicts.is_empty());
    assert!(matches!(present.merged, ScalarAssertion::Present(_)));
}

#[test]
fn scalar_assertion_equals_present_and_oneof_intersections_compose() {
    let mut conflicts = Vec::new();
    let equals_present = ScalarAssertion::resolve(
        "edition",
        vec![
            (
                prov("equals"),
                ScalarAssertion::Equals("2024".to_owned(), "equals".to_owned()),
            ),
            (
                prov("present"),
                ScalarAssertion::Present("present".to_owned()),
            ),
        ],
        &mut conflicts,
    )
    .expect("equals should satisfy present");
    assert!(matches!(
        equals_present.merged,
        ScalarAssertion::Equals(ref value, _) if value == "2024"
    ));

    let nonempty = ScalarAssertion::resolve(
        "edition",
        vec![
            (
                prov("left"),
                ScalarAssertion::OneOf(
                    BTreeSet::from(["2021".to_owned(), "2024".to_owned()]),
                    "left".to_owned(),
                ),
            ),
            (
                prov("right"),
                ScalarAssertion::OneOf(BTreeSet::from(["2024".to_owned()]), "right".to_owned()),
            ),
        ],
        &mut conflicts,
    )
    .expect("overlapping oneof sets should resolve");
    assert!(matches!(
        nonempty.merged,
        ScalarAssertion::OneOf(ref values, _) if values == &BTreeSet::from(["2024".to_owned()])
    ));

    let empty = ScalarAssertion::resolve(
        "edition",
        vec![
            (
                prov("left"),
                ScalarAssertion::OneOf(BTreeSet::from(["2021".to_owned()]), "left".to_owned()),
            ),
            (
                prov("right"),
                ScalarAssertion::OneOf(BTreeSet::from(["2024".to_owned()]), "right".to_owned()),
            ),
        ],
        &mut conflicts,
    );
    assert!(empty.is_none());
    assert_eq!(
        conflicts.last().map(|conflict| conflict.reason.as_str()),
        Some("scalar-disagree")
    );
}

#[test]
fn scalar_assertion_bounds_keep_strongest_values() {
    let mut conflicts = Vec::new();
    let floor = ScalarAssertion::resolve(
        "threshold",
        vec![
            (
                prov("low"),
                ScalarAssertion::AtLeast(5_u64, "low".to_owned()),
            ),
            (
                prov("high"),
                ScalarAssertion::AtLeast(10_u64, "high".to_owned()),
            ),
        ],
        &mut conflicts,
    )
    .expect("compatible floors should resolve");
    assert!(matches!(floor.merged, ScalarAssertion::AtLeast(10, _)));

    let ceiling = ScalarAssertion::resolve(
        "threshold",
        vec![
            (
                prov("high"),
                ScalarAssertion::AtMost(10_u64, "high".to_owned()),
            ),
            (
                prov("low"),
                ScalarAssertion::AtMost(5_u64, "low".to_owned()),
            ),
        ],
        &mut conflicts,
    )
    .expect("compatible ceilings should resolve");
    assert!(matches!(ceiling.merged, ScalarAssertion::AtMost(5, _)));
}

#[test]
fn scalar_assertion_range_participates_in_bounds_and_filters_oneof() {
    let mut conflicts = Vec::new();
    let resolved = ScalarAssertion::resolve(
        "threshold",
        vec![
            (
                prov("range"),
                ScalarAssertion::Range(5_u64, 20_u64, "range".to_owned()),
            ),
            (
                prov("floor"),
                ScalarAssertion::AtLeast(10_u64, "floor".to_owned()),
            ),
            (
                prov("allowed"),
                ScalarAssertion::OneOf(
                    BTreeSet::from([1_u64, 10_u64, 30_u64]),
                    "allowed".to_owned(),
                ),
            ),
        ],
        &mut conflicts,
    )
    .expect("range, floor, and oneof should compose");

    assert!(conflicts.is_empty());
    assert!(matches!(
        resolved.merged,
        ScalarAssertion::OneOf(ref values, _) if values == &BTreeSet::from([10_u64])
    ));
}

#[test]
fn scalar_assertion_ordered_operation_on_unordered_value_conflicts() {
    let mut conflicts = Vec::new();
    let resolved = ScalarAssertion::resolve(
        "edition",
        vec![(
            prov("ordered"),
            ScalarAssertion::AtLeast("2024".to_owned(), "ordered".to_owned()),
        )],
        &mut conflicts,
    );

    assert!(resolved.is_none());
    assert_eq!(conflicts.len(), 1);
    assert_eq!(
        conflicts.first().expect("one conflict").reason,
        "scalar-order-unsupported"
    );
}

#[test]
fn scalar_assertion_ordered_range_conflicts_when_empty() {
    let mut conflicts = Vec::new();
    let resolved = ScalarAssertion::resolve(
        "threshold",
        vec![
            (
                prov("floor"),
                ScalarAssertion::AtLeast(10_u64, "floor".to_owned()),
            ),
            (
                prov("ceiling"),
                ScalarAssertion::AtMost(5_u64, "ceiling".to_owned()),
            ),
        ],
        &mut conflicts,
    );

    assert!(resolved.is_none());
    assert_eq!(conflicts.len(), 1);
    assert_eq!(
        conflicts.first().expect("one conflict").reason,
        "scalar-disagree"
    );
}

#[test]
fn push_conflict_with_no_contributors_does_not_emit_conflict() {
    let mut conflicts = Vec::new();
    let items: Vec<Contributor> = Vec::new();

    push_conflict(
        "edition",
        "scalar-disagree",
        &items,
        Clone::clone,
        &mut conflicts,
    );

    assert!(conflicts.is_empty());
}

#[test]
fn exact_list_conflict_uses_stable_contributor_text() {
    let mut conflicts = Vec::new();
    let resolved = resolve_exact_list(
        "ignore",
        vec![
            (prov("left"), (vec!["target".to_owned()], "left".to_owned())),
            (prov("right"), (vec!["dist".to_owned()], "right".to_owned())),
        ],
        &mut conflicts,
    );

    assert!(resolved.is_none());
    assert_eq!(
        conflicts.first().expect("one conflict").contributors,
        vec![
            (prov("left"), "exact [target]".to_owned()),
            (prov("right"), "exact [dist]".to_owned())
        ]
    );
}

#[test]
fn dotted_version_orders_by_numeric_tuple() {
    assert!(DottedVersion::new("1.10") > DottedVersion::new("1.9"));
    assert!(DottedVersion::new("1.85.1") > DottedVersion::new("1.85.0"));
}
use schemars as _;
