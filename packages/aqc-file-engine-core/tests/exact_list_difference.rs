use std::collections::BTreeMap;

use aqc_file_engine_core::{
    ListRequirements, Provenance, apply_list_requirements, exact_list_difference, resolve_list,
};
use schemars as _;
use serde as _;

fn strings(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| (*value).to_owned()).collect()
}

#[test]
fn exact_list_difference_covers_membership_duplicates_empty_values_and_order() {
    let equal = exact_list_difference(&strings(&["a", "b"]), &strings(&["a", "b"]));
    assert!(equal.is_empty());

    let replacement = exact_list_difference(&strings(&["b"]), &strings(&["a"]));
    assert_eq!(
        replacement.missing(),
        &BTreeMap::from([("a".to_owned(), 1)])
    );
    assert_eq!(
        replacement.unexpected(),
        &BTreeMap::from([("b".to_owned(), 1)])
    );
    assert!(!replacement.order_mismatch());

    let duplicate = exact_list_difference(&strings(&["", "a", "a"]), &strings(&["", "a"]));
    assert!(duplicate.missing().is_empty());
    assert_eq!(
        duplicate.unexpected(),
        &BTreeMap::from([("a".to_owned(), 1)])
    );

    let order = exact_list_difference(&strings(&["b", "a"]), &strings(&["a", "b"]));
    assert!(order.missing().is_empty());
    assert!(order.unexpected().is_empty());
    assert!(order.order_mismatch());
    assert!(!order.is_empty());
}

#[test]
fn exact_list_difference_orders_distinct_members_lexically() {
    let difference = exact_list_difference(&strings(&["z", "b", "z"]), &strings(&["y", "a", "y"]));
    assert_eq!(
        difference.missing().keys().cloned().collect::<Vec<_>>(),
        strings(&["a", "y"])
    );
    assert_eq!(
        difference.unexpected().keys().cloned().collect::<Vec<_>>(),
        strings(&["b", "z"])
    );
}

#[test]
fn apply_list_requirements_uses_exact_then_contains_then_excludes() {
    let mut requirement = ListRequirements::default();
    let _ = requirement
        .contains
        .insert("c".to_owned(), "contains".to_owned());
    let _ = requirement
        .excludes
        .insert("b".to_owned(), "excludes".to_owned());
    let mut conflicts = Vec::new();
    let resolved = resolve_list(
        "values",
        vec![(
            Provenance {
                policy: "policy".to_owned(),
            },
            requirement,
        )],
        &mut conflicts,
    );

    assert!(conflicts.is_empty());
    assert_eq!(
        apply_list_requirements(&strings(&["a", "b"]), &resolved),
        strings(&["a", "c"])
    );
}
