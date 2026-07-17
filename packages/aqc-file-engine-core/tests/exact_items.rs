use aqc_file_engine_core::{
    ConflictEntry, FileKeyRequirement, ItemRequirements, KeyedItem, Provenance, ScalarAssertion,
    resolve_items, resolve_key_membership,
};
use schemars as _;
use serde as _;

type Item = KeyedItem<u8>;

fn prov(policy: &str) -> Provenance {
    Provenance {
        policy: policy.to_owned(),
    }
}

fn item(key: &str, value: u8) -> Item {
    Item {
        file_key: key.to_owned(),
        value,
    }
}

const fn requirements(
    required: Vec<(Item, String)>,
    forbidden: Vec<(Item, String)>,
    exact: Option<(Vec<Item>, String)>,
) -> ItemRequirements<Item> {
    ItemRequirements {
        required,
        forbidden,
        allowed: None,
        exact,
    }
}

fn resolve(
    inputs: Vec<(Provenance, ItemRequirements<Item>)>,
) -> (
    aqc_file_engine_core::ResolvedItemRequirements<Item>,
    Vec<ConflictEntry>,
) {
    let mut conflicts = Vec::new();
    let resolved = resolve_items("items", inputs, &mut conflicts);
    (resolved, conflicts)
}

#[test]
fn required_items_allow_additional_identities() {
    let (resolved, conflicts) = resolve(vec![(
        prov("required"),
        requirements(vec![(item("a", 1), "need a".to_owned())], Vec::new(), None),
    )]);

    assert!(conflicts.is_empty());
    assert_eq!(resolved.required.len(), 1);
    assert!(resolved.exact.is_none());
}

#[test]
fn forbidden_items_allow_other_identities() {
    let (resolved, conflicts) = resolve(vec![(
        prov("forbidden"),
        requirements(Vec::new(), vec![(item("a", 0), "no a".to_owned())], None),
    )]);

    assert!(conflicts.is_empty());
    assert_eq!(resolved.forbidden.len(), 1);
    assert!(resolved.exact.is_none());
}

#[test]
fn allowed_items_intersect_without_becoming_required() {
    let (resolved, conflicts) = resolve(vec![
        (
            prov("broad"),
            ItemRequirements {
                allowed: Some((vec![item("a", 1), item("b", 2)], "allow a or b".to_owned())),
                ..ItemRequirements::default()
            },
        ),
        (
            prov("narrow"),
            ItemRequirements {
                allowed: Some((vec![item("b", 9)], "allow b".to_owned())),
                ..ItemRequirements::default()
            },
        ),
    ]);

    assert!(conflicts.is_empty());
    assert!(resolved.required.is_empty());
    let allowed = resolved.allowed.expect("allowed identities must resolve");
    assert_eq!(
        allowed.identities.into_iter().collect::<Vec<_>>(),
        vec!["b"]
    );
    assert_eq!(allowed.collected.len(), 2);
}

#[test]
fn rejected_identity_uses_only_the_allowed_contributors_that_exclude_it() {
    let (resolved, conflicts) = resolve(vec![
        (
            prov("broad"),
            ItemRequirements {
                allowed: Some((vec![item("a", 1), item("b", 2)], "broad".to_owned())),
                ..ItemRequirements::default()
            },
        ),
        (
            prov("narrow"),
            ItemRequirements {
                allowed: Some((vec![item("a", 1)], "narrow".to_owned())),
                ..ItemRequirements::default()
            },
        ),
    ]);

    assert!(conflicts.is_empty());
    let membership = resolved
        .membership()
        .expect("allowed membership must resolve");
    assert_eq!(
        membership.message_for_rejected(|candidate| candidate.file_key == "b"),
        "narrow"
    );
    assert_eq!(
        membership.attribution_for_rejected(|candidate| candidate.file_key == "b"),
        vec![prov("narrow")]
    );
}

#[test]
fn allowed_conflict_survives_failed_required_value_composition() {
    let (_, conflicts) = resolve(vec![
        (
            prov("allowed"),
            ItemRequirements {
                allowed: Some((vec![item("a", 1)], "only a".to_owned())),
                ..ItemRequirements::default()
            },
        ),
        (
            prov("required-one"),
            requirements(vec![(item("b", 1), "one".to_owned())], Vec::new(), None),
        ),
        (
            prov("required-two"),
            requirements(vec![(item("b", 2), "two".to_owned())], Vec::new(), None),
        ),
    ]);

    assert!(conflicts.iter().any(|conflict| {
        conflict.key == "items.b" && conflict.reason == "allowed-items-reject-required-item"
    }));
}

#[test]
fn required_item_outside_allowed_conflicts() {
    let (_, conflicts) = resolve(vec![
        (
            prov("allowed"),
            ItemRequirements {
                allowed: Some((vec![item("a", 0)], "only a is allowed".to_owned())),
                ..ItemRequirements::default()
            },
        ),
        (
            prov("required"),
            requirements(vec![(item("b", 2), "need b".to_owned())], Vec::new(), None),
        ),
    ]);

    assert!(conflicts.iter().any(|conflict| {
        conflict.key == "items.b"
            && conflict.reason == "allowed-items-reject-required-item"
            && conflict.contributors
                == vec![
                    (prov("allowed"), "only a is allowed".to_owned()),
                    (prov("required"), "required".to_owned()),
                ]
    }));
}

#[test]
fn exact_item_outside_allowed_conflicts() {
    let (_, conflicts) = resolve(vec![
        (
            prov("allowed"),
            ItemRequirements {
                allowed: Some((vec![item("a", 0)], "only a is allowed".to_owned())),
                ..ItemRequirements::default()
            },
        ),
        (
            prov("exact"),
            requirements(
                Vec::new(),
                Vec::new(),
                Some((vec![item("b", 2)], "exact b".to_owned())),
            ),
        ),
    ]);

    assert!(conflicts.iter().any(|conflict| {
        conflict.key == "items.b"
            && conflict.reason == "allowed-items-reject-required-item"
            && conflict.contributors
                == vec![
                    (prov("allowed"), "only a is allowed".to_owned()),
                    (prov("exact"), "exact b".to_owned()),
                ]
    }));
}

#[test]
fn required_and_exact_item_outside_allowed_produce_one_conflict() {
    let (_, conflicts) = resolve(vec![
        (
            prov("allowed"),
            ItemRequirements {
                allowed: Some((vec![item("a", 0)], "only a is allowed".to_owned())),
                ..ItemRequirements::default()
            },
        ),
        (
            prov("exact"),
            requirements(
                Vec::new(),
                Vec::new(),
                Some((vec![item("b", 2)], "exact b".to_owned())),
            ),
        ),
        (
            prov("required"),
            requirements(vec![(item("b", 2), "need b".to_owned())], Vec::new(), None),
        ),
    ]);

    let matching = conflicts
        .iter()
        .filter(|conflict| {
            conflict.key == "items.b" && conflict.reason == "allowed-items-reject-required-item"
        })
        .collect::<Vec<_>>();
    assert_eq!(matching.len(), 1);
    assert_eq!(
        matching
            .first()
            .expect("one allowed-membership conflict")
            .contributors
            .len(),
        3
    );
}

#[test]
fn forbidden_item_inside_allowed_is_compatible() {
    let (resolved, conflicts) = resolve(vec![
        (
            prov("allowed"),
            ItemRequirements {
                allowed: Some((vec![item("a", 0)], "a may exist".to_owned())),
                ..ItemRequirements::default()
            },
        ),
        (
            prov("forbidden"),
            requirements(Vec::new(), vec![(item("a", 0), "no a".to_owned())], None),
        ),
    ]);

    assert!(conflicts.is_empty());
    assert!(resolved.forbidden.contains_key("a"));
}

#[test]
fn empty_exact_resolves_to_an_empty_complete_collection() {
    let (resolved, conflicts) = resolve(vec![(
        prov("exact"),
        requirements(
            Vec::new(),
            Vec::new(),
            Some((Vec::new(), "none".to_owned())),
        ),
    )]);

    assert!(conflicts.is_empty());
    assert!(resolved.exact.is_some());
    let Some(exact) = resolved.exact else { return };
    assert!(exact.identities.is_empty());
    assert!(exact.items.is_empty());
    assert_eq!(exact.collected.len(), 1);
}

#[test]
fn compatible_required_and_exact_values_compose_with_full_attribution() {
    let (resolved, conflicts) = resolve(vec![
        (
            prov("exact"),
            requirements(
                Vec::new(),
                Vec::new(),
                Some((vec![item("a", 1)], "only a".to_owned())),
            ),
        ),
        (
            prov("required"),
            requirements(vec![(item("a", 1), "need a".to_owned())], Vec::new(), None),
        ),
    ]);

    assert!(conflicts.is_empty());
    assert!(resolved.required.contains_key("a"));
    let Some(required) = resolved.required.get("a") else {
        return;
    };
    assert_eq!(required.merged, item("a", 1));
    assert_eq!(required.collected.len(), 1);
    let exact_count = resolved
        .exact
        .as_ref()
        .and_then(|exact| exact.items.get("a"))
        .map_or(0, |item| item.collected.len());
    assert_eq!(exact_count, 2);
}

#[test]
fn required_identity_outside_exact_conflicts_with_both_contributors() {
    let (_, conflicts) = resolve(vec![
        (
            prov("exact"),
            requirements(
                Vec::new(),
                Vec::new(),
                Some((vec![item("a", 1)], "only a".to_owned())),
            ),
        ),
        (
            prov("required"),
            requirements(vec![(item("b", 2), "need b".to_owned())], Vec::new(), None),
        ),
    ]);

    assert_eq!(conflicts.len(), 1);
    assert!(!conflicts.is_empty());
    let Some(conflict) = conflicts.first() else {
        return;
    };
    assert_eq!(conflict.key, "items.b");
    assert_eq!(conflict.reason, "exact-items-reject-unlisted-required-item");
    assert_eq!(
        conflict.contributors,
        vec![
            (prov("exact"), "only a".to_owned()),
            (prov("required"), "required".to_owned())
        ]
    );
}

#[test]
fn forbidden_identity_inside_exact_conflicts_with_both_contributors() {
    let (_, conflicts) = resolve(vec![
        (
            prov("exact"),
            requirements(
                Vec::new(),
                Vec::new(),
                Some((vec![item("a", 1)], "only a".to_owned())),
            ),
        ),
        (
            prov("forbidden"),
            requirements(Vec::new(), vec![(item("a", 0), "no a".to_owned())], None),
        ),
    ]);

    assert!(conflicts.iter().any(|conflict| {
        conflict.key == "items.a"
            && conflict.reason == "exact-item-is-forbidden"
            && conflict.contributors
                == vec![
                    (prov("exact"), "only a".to_owned()),
                    (prov("forbidden"), "forbidden".to_owned()),
                ]
    }));
}

#[test]
fn agreeing_exact_assertions_merge_independent_of_order() {
    let (resolved, conflicts) = resolve(vec![
        (
            prov("left"),
            requirements(
                Vec::new(),
                Vec::new(),
                Some((vec![item("a", 1), item("b", 2)], "left exact".to_owned())),
            ),
        ),
        (
            prov("right"),
            requirements(
                Vec::new(),
                Vec::new(),
                Some((vec![item("b", 2), item("a", 1)], "right exact".to_owned())),
            ),
        ),
    ]);

    assert!(conflicts.is_empty());
    assert!(resolved.exact.is_some());
    let Some(exact) = resolved.exact else { return };
    assert_eq!(
        exact.identities.into_iter().collect::<Vec<_>>(),
        vec!["a", "b"]
    );
    assert_eq!(exact.collected.len(), 2);
    assert_eq!(
        exact.items.get("a").map_or(0, |item| item.collected.len()),
        2
    );
    assert_eq!(
        exact.items.get("b").map_or(0, |item| item.collected.len()),
        2
    );
}

#[test]
fn differing_exact_identity_sets_conflict_with_exact_messages() {
    let (_, conflicts) = resolve(vec![
        (
            prov("left"),
            requirements(
                Vec::new(),
                Vec::new(),
                Some((vec![item("a", 1)], "only a".to_owned())),
            ),
        ),
        (
            prov("right"),
            requirements(
                Vec::new(),
                Vec::new(),
                Some((vec![item("b", 2)], "only b".to_owned())),
            ),
        ),
    ]);

    assert!(conflicts.iter().any(|conflict| {
        conflict.key == "items"
            && conflict.reason == "exact-item-identities-disagree"
            && conflict.contributors
                == vec![
                    (prov("left"), "only a".to_owned()),
                    (prov("right"), "only b".to_owned()),
                ]
    }));
}

#[test]
fn scalar_value_constraints_participate_in_key_membership_merge() {
    let mut membership = ItemRequirements {
        required: Vec::new(),
        forbidden: Vec::new(),
        allowed: None,
        exact: Some((Vec::new(), "no keys".to_owned())),
    };
    ScalarAssertion::Equals(1_u8, "value required".to_owned())
        .constrain_file_key("value", &mut membership);

    let mut conflicts = Vec::new();
    let _ = resolve_items(
        "settings",
        vec![(prov("policy"), membership)],
        &mut conflicts,
    );

    assert!(conflicts.iter().any(|conflict| {
        conflict.key == "settings.value"
            && conflict.reason == "exact-items-reject-unlisted-required-item"
    }));
}

#[test]
fn absent_scalar_becomes_forbidden_key_membership() {
    let mut membership = ItemRequirements {
        required: vec![(
            KeyedItem {
                file_key: "value".to_owned(),
                value: (),
            },
            "value required".to_owned(),
        )],
        forbidden: Vec::new(),
        allowed: None,
        exact: None,
    };
    ScalarAssertion::<u8>::Absent("value absent".to_owned())
        .constrain_file_key("value", &mut membership);

    let mut conflicts = Vec::new();
    let _ = resolve_items(
        "settings",
        vec![(prov("policy"), membership)],
        &mut conflicts,
    );

    assert!(conflicts.iter().any(|conflict| {
        conflict.key == "settings.value" && conflict.reason == "item-required-and-forbidden"
    }));
}

#[test]
fn derived_key_constraints_do_not_become_reconciliation_membership() {
    let explicit = ItemRequirements::<KeyedItem<()>>::default();
    let mut constrained = ItemRequirements::default();
    ScalarAssertion::<u8>::Present("value required".to_owned())
        .constrain_file_key("value", &mut constrained);
    let mut conflicts = Vec::new();

    let resolved = resolve_key_membership(
        "settings",
        vec![(prov("policy"), explicit)],
        vec![(prov("policy"), constrained)],
        &mut conflicts,
    );

    assert!(conflicts.is_empty());
    assert!(resolved.required.is_empty());
}

#[test]
fn derived_key_constraints_are_checked_against_explicit_membership() {
    let explicit = ItemRequirements {
        allowed: None,
        exact: Some((Vec::new(), "no settings".to_owned())),
        ..ItemRequirements::default()
    };
    let mut derived = ItemRequirements::default();
    ScalarAssertion::<u8>::Present("value required".to_owned())
        .constrain_file_key("value", &mut derived);
    let mut conflicts = Vec::new();

    let _ = resolve_key_membership(
        "settings",
        vec![(prov("policy"), explicit)],
        vec![(prov("policy"), derived)],
        &mut conflicts,
    );

    assert!(conflicts.iter().any(|conflict| {
        conflict.key == "settings.value"
            && conflict.reason == "exact-items-reject-unlisted-required-item"
    }));
}

#[test]
fn duplicate_same_identity_exact_items_compose_when_values_agree() {
    let (resolved, conflicts) = resolve(vec![(
        prov("exact"),
        requirements(
            Vec::new(),
            Vec::new(),
            Some((vec![item("a", 1), item("a", 1)], "duplicates".to_owned())),
        ),
    )]);

    assert!(conflicts.is_empty());
    assert!(resolved.required.is_empty());
    assert!(resolved.exact.is_some());
    let Some(exact) = resolved.exact else { return };
    assert_eq!(exact.identities.len(), 1);
    assert_eq!(
        exact.items.get("a").map_or(0, |item| item.collected.len()),
        2
    );
}

#[test]
fn same_identity_value_disagreement_uses_normal_item_composition() {
    let (_, conflicts) = resolve(vec![
        (
            prov("exact"),
            requirements(
                Vec::new(),
                Vec::new(),
                Some((vec![item("a", 1)], "a is one".to_owned())),
            ),
        ),
        (
            prov("required"),
            requirements(
                vec![(item("a", 2), "a is two".to_owned())],
                Vec::new(),
                None,
            ),
        ),
    ]);

    assert!(
        conflicts
            .iter()
            .any(|conflict| conflict.key == "items.a" && conflict.reason == "set-key-disagree")
    );
    assert!(conflicts.iter().any(|conflict| {
        conflict.contributors
            == vec![
                (prov("exact"), "a is one".to_owned()),
                (prov("required"), "a is two".to_owned()),
            ]
    }));
}
