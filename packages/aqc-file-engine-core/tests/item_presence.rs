#![allow(
    clippy::expect_used,
    reason = "Tests fail loudly when resolved requirement invariants are broken."
)]

use std::collections::BTreeSet;

use aqc_file_engine_core::{
    ConflictEntry, FileItemRequirement, ItemAssertionInput, ItemRequirements, KeyedItem,
    Provenance, RequiredItemResolution, compose_item_by, item_presence_difference, resolve_items,
};
use schemars as _;
use serde as _;

#[derive(Debug, Clone, PartialEq, Eq)]
struct TestItem {
    key: String,
    value: u8,
}

impl TestItem {
    fn new(key: &str, value: u8) -> Self {
        Self {
            key: key.to_owned(),
            value,
        }
    }
}

impl FileItemRequirement for TestItem {
    type Identity = String;

    fn merge_identity(&self) -> Self::Identity {
        self.key.clone()
    }

    fn compose_item(
        key: &str,
        items: Vec<ItemAssertionInput<Self>>,
        conflicts: &mut Vec<ConflictEntry>,
    ) -> Option<RequiredItemResolution<Self>> {
        compose_item_by(key, items, |item| item.value, conflicts)
    }
}

fn provenance(policy: &str) -> Provenance {
    Provenance {
        policy: policy.to_owned(),
    }
}

fn resolve(
    inputs: Vec<(Provenance, ItemRequirements<TestItem>)>,
) -> Result<aqc_file_engine_core::ResolvedItemRequirements<TestItem>, Vec<ConflictEntry>> {
    let mut conflicts = Vec::new();
    let resolved = resolve_items("items", inputs, &mut conflicts);
    if conflicts.is_empty() {
        Ok(resolved)
    } else {
        Err(conflicts)
    }
}

#[test]
fn item_requirements_map_preserves_every_collection_and_message() {
    let requirements = ItemRequirements {
        required: vec![(1_u8, "required".to_owned())],
        forbidden: vec![(2_u8, "forbidden".to_owned())],
        allowed: Some((vec![5_u8], "allowed".to_owned())),
        exact: Some((vec![3_u8, 4_u8], "exact".to_owned())),
    };

    let mapped = requirements.map(|item| KeyedItem {
        file_key: format!("key-{item}"),
        value: (),
    });

    let required = mapped.required.first().expect("one mapped required item");
    let forbidden = mapped.forbidden.first().expect("one mapped forbidden item");
    let (allowed, allowed_message) = mapped.allowed.expect("mapped allowed collection");
    assert_eq!(required.0.file_key, "key-1");
    assert_eq!(required.1, "required");
    assert_eq!(forbidden.0.file_key, "key-2");
    assert_eq!(forbidden.1, "forbidden");
    assert_eq!(
        allowed.first().expect("one mapped allowed item").file_key,
        "key-5"
    );
    assert_eq!(allowed_message, "allowed");
    let (exact, message) = mapped.exact.expect("mapped exact collection");
    assert_eq!(
        exact
            .iter()
            .map(|item| item.file_key.as_str())
            .collect::<Vec<_>>(),
        vec!["key-3", "key-4"]
    );
    assert_eq!(message, "exact");
}

#[test]
fn item_requirements_map_preserves_an_absent_exact_collection() {
    let mapped = ItemRequirements {
        required: vec![(1_u8, "required".to_owned())],
        forbidden: Vec::new(),
        allowed: None,
        exact: None,
    }
    .map(u16::from);

    assert_eq!(mapped.required, vec![(1_u16, "required".to_owned())]);
    assert!(mapped.exact.is_none());
}

#[test]
fn presence_difference_reports_missing_for_required_and_exact_once() {
    let requirements = resolve(vec![(
        provenance("policy"),
        ItemRequirements {
            required: vec![(TestItem::new("shared", 1), "required".to_owned())],
            forbidden: Vec::new(),
            allowed: None,
            exact: Some((
                vec![TestItem::new("shared", 1), TestItem::new("exact", 2)],
                "exact".to_owned(),
            )),
        },
    )])
    .expect("compatible exact and required members must resolve");

    let current = BTreeSet::new();
    let difference = item_presence_difference(&current, &requirements);
    let missing = difference
        .missing
        .iter()
        .map(|(identity, _)| identity.as_str())
        .collect::<Vec<_>>();

    assert_eq!(missing, vec!["exact", "shared"]);
    assert!(difference.forbidden.is_empty());
    assert!(difference.unexpected.is_empty());
}

#[test]
fn allowed_items_reject_extras_without_requiring_allowed_members() {
    let requirements = resolve(vec![(
        provenance("policy"),
        ItemRequirements {
            allowed: Some((vec![TestItem::new("optional", 1)], "closed".to_owned())),
            ..ItemRequirements::default()
        },
    )])
    .expect("an allowed collection must resolve");

    let current = BTreeSet::from(["extra".to_owned()]);
    let difference = item_presence_difference(&current, &requirements);

    assert!(difference.missing.is_empty());
    assert_eq!(difference.unexpected, vec![&"extra".to_owned()]);
}

#[test]
fn presence_difference_reports_present_forbidden_and_preserves_attribution() {
    let first = provenance("first");
    let second = provenance("second");
    let requirements = resolve(vec![
        (
            second.clone(),
            ItemRequirements {
                forbidden: vec![(TestItem::new("blocked", 1), "second".to_owned())],
                ..ItemRequirements::default()
            },
        ),
        (
            first.clone(),
            ItemRequirements {
                forbidden: vec![(TestItem::new("blocked", 1), "first".to_owned())],
                ..ItemRequirements::default()
            },
        ),
    ])
    .expect("compatible forbidden requirements must resolve");

    let current = BTreeSet::from(["blocked".to_owned()]);
    let difference = item_presence_difference(&current, &requirements);

    assert_eq!(difference.forbidden.len(), 1);
    let (identity, resolved) = difference
        .forbidden
        .first()
        .expect("one present forbidden item");
    assert_eq!(*identity, "blocked");
    assert_eq!(resolved.attribution(), vec![first, second]);
}

#[test]
fn exact_and_forbidden_membership_report_one_rejection_per_identity() {
    let resolved = resolve(vec![(
        provenance("policy"),
        ItemRequirements {
            required: Vec::new(),
            forbidden: vec![(TestItem::new("a", 1), "forbidden".to_owned())],
            allowed: None,
            exact: Some((Vec::new(), "exact".to_owned())),
        },
    )])
    .expect("compatible exact and forbidden requirements must resolve");
    let current = BTreeSet::from(["a".to_owned()]);

    let difference = item_presence_difference(&current, &resolved);

    assert_eq!(difference.forbidden.len(), 1);
    assert!(difference.unexpected.is_empty());
}

#[test]
fn presence_difference_reports_unexpected_exact_members_in_identity_order() {
    let requirements = resolve(vec![(
        provenance("policy"),
        ItemRequirements {
            allowed: None,
            exact: Some((vec![TestItem::new("allowed", 1)], "exact".to_owned())),
            ..ItemRequirements::default()
        },
    )])
    .expect("exact requirements must resolve");

    let current = BTreeSet::from([
        "z-extra".to_owned(),
        "allowed".to_owned(),
        "a-extra".to_owned(),
    ]);
    let difference = item_presence_difference(&current, &requirements);

    assert_eq!(
        difference
            .unexpected
            .iter()
            .map(|identity| identity.as_str())
            .collect::<Vec<_>>(),
        vec!["a-extra", "z-extra"]
    );
}

#[test]
fn exact_empty_reports_every_present_identity_as_unexpected() {
    let requirements = resolve(vec![(
        provenance("policy"),
        ItemRequirements {
            allowed: None,
            exact: Some((Vec::new(), "exact empty".to_owned())),
            ..ItemRequirements::default()
        },
    )])
    .expect("an exact empty collection must resolve");
    let current = BTreeSet::from(["one".to_owned(), "two".to_owned()]);

    let difference = item_presence_difference(&current, &requirements);

    assert_eq!(difference.unexpected.len(), 2);
    assert!(difference.missing.is_empty());
}

#[test]
fn duplicate_exact_identities_are_returned_once() {
    let requirements = resolve(vec![(
        provenance("policy"),
        ItemRequirements {
            allowed: None,
            exact: Some((
                vec![TestItem::new("same", 1), TestItem::new("same", 1)],
                "duplicates".to_owned(),
            )),
            ..ItemRequirements::default()
        },
    )])
    .expect("compatible duplicate identities must resolve");

    let current = BTreeSet::new();
    let difference = item_presence_difference(&current, &requirements);

    assert_eq!(difference.missing.len(), 1);
    assert_eq!(
        *difference
            .missing
            .first()
            .expect("one duplicate identity")
            .0,
        "same"
    );
}

#[test]
fn compatible_exact_sets_compose_and_incompatible_sets_conflict() {
    let exact = |items: Vec<TestItem>| ItemRequirements {
        allowed: None,
        exact: Some((items, "exact".to_owned())),
        ..ItemRequirements::default()
    };
    let compatible = resolve(vec![
        (
            provenance("first"),
            exact(vec![TestItem::new("a", 1), TestItem::new("b", 2)]),
        ),
        (
            provenance("second"),
            exact(vec![TestItem::new("b", 2), TestItem::new("a", 1)]),
        ),
    ]);
    assert!(compatible.is_ok());

    let incompatible = resolve(vec![
        (provenance("first"), exact(vec![TestItem::new("a", 1)])),
        (provenance("second"), exact(vec![TestItem::new("b", 2)])),
    ])
    .expect_err("incompatible exact identity sets must conflict");
    assert!(
        incompatible
            .iter()
            .any(|conflict| conflict.reason == "exact-item-identities-disagree")
    );
}

#[test]
fn required_outside_exact_and_forbidden_inside_exact_conflict() {
    let conflicts = resolve(vec![
        (
            provenance("exact"),
            ItemRequirements {
                allowed: None,
                exact: Some((vec![TestItem::new("inside", 1)], "exact".to_owned())),
                ..ItemRequirements::default()
            },
        ),
        (
            provenance("required"),
            ItemRequirements {
                required: vec![(TestItem::new("outside", 2), "required".to_owned())],
                ..ItemRequirements::default()
            },
        ),
        (
            provenance("forbidden"),
            ItemRequirements {
                forbidden: vec![(TestItem::new("inside", 1), "forbidden".to_owned())],
                ..ItemRequirements::default()
            },
        ),
    ])
    .expect_err("required outside exact and forbidden inside exact must conflict");

    assert!(
        conflicts
            .iter()
            .any(|conflict| conflict.reason == "exact-items-reject-unlisted-required-item")
    );
    assert!(
        conflicts
            .iter()
            .any(|conflict| conflict.reason == "exact-item-is-forbidden")
    );
}
