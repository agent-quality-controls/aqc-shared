use std::collections::{BTreeMap, BTreeSet};

use aqc_file_engine_core::{FileEngine as _, Finding};
use aqc_json_engine_core as _;
use aqc_json_file_engine::{
    ConfigScalar, ForbiddenGlobRequirements, ItemRequirements, JsonFileEngine,
    JsonFileRequirements, JsonPath, JsonStringGlob, KeyedItem, ListRequirements, Provenance,
    ScalarAssertion,
};
use globset as _;

fn provenance(policy: &str) -> Provenance {
    Provenance {
        policy: policy.to_owned(),
    }
}

#[test]
fn scalar_and_string_list_requirements_reconcile_nested_strict_json() {
    let scalar = JsonPath::new("nested").child("enabled");
    let list = JsonPath::new("plugins");
    let requirement = JsonFileRequirements {
        scalar_values: BTreeMap::from([(
            scalar,
            ScalarAssertion::Equals(ConfigScalar::Bool(true), "Enable it.".to_owned()),
        )]),
        string_lists: BTreeMap::from([(
            list,
            ListRequirements {
                contains: BTreeMap::from([("required".to_owned(), "Require it.".to_owned())]),
                ..ListRequirements::default()
            },
        )]),
        ..JsonFileRequirements::default()
    };
    let resolved = JsonFileRequirements::merge(vec![(provenance("policy"), requirement)])
        .expect("requirements must resolve");
    let output = JsonFileEngine::reconcile(Some(br#"{"kept":1}"#), &resolved);
    let rendered = String::from_utf8(output.expected_bytes).expect("JSON is UTF-8");
    assert!(rendered.contains("\"kept\":1"));
    assert!(rendered.contains("\"enabled\": true"));
    assert!(rendered.contains("\"required\""));
    assert_eq!(output.findings.len(), 2);
}

#[test]
fn forbidden_glob_reports_each_selector_and_removes_matches() {
    let path = JsonPath::new("ignore");
    let requirement = JsonFileRequirements {
        forbidden_string_list_globs: BTreeMap::from([(
            path,
            ForbiddenGlobRequirements {
                globs: vec![(
                    JsonStringGlob {
                        glob: "**/*.generated.ts".to_owned(),
                    },
                    "Generated files may not be ignored.".to_owned(),
                )],
            },
        )]),
        ..JsonFileRequirements::default()
    };
    let resolved = JsonFileRequirements::merge(vec![(provenance("policy"), requirement)])
        .expect("requirements must resolve");
    let output = JsonFileEngine::reconcile(
        Some(br#"{"ignore":["src/a.generated.ts","src/keep.ts"]}"#),
        &resolved,
    );
    assert_eq!(output.findings.len(), 1);
    let finding = format!("{:?}", output.findings.first());
    assert!(finding.contains("src/a.generated.ts"));
    assert!(finding.contains("policy"));
    let rendered = String::from_utf8(output.expected_bytes).expect("JSON is UTF-8");
    assert!(!rendered.contains("a.generated"));
    assert!(rendered.contains("keep.ts"));
}

#[test]
fn scalar_and_string_list_at_same_path_conflict_with_provenance() {
    let path = JsonPath::new("value");
    let scalar = JsonFileRequirements {
        scalar_values: BTreeMap::from([(
            path.clone(),
            ScalarAssertion::Equals(ConfigScalar::Bool(true), "scalar".to_owned()),
        )]),
        ..JsonFileRequirements::default()
    };
    let list = JsonFileRequirements {
        string_lists: BTreeMap::from([(
            path,
            ListRequirements {
                exact: Some((vec!["item".to_owned()], "list".to_owned())),
                ..ListRequirements::default()
            },
        )]),
        ..JsonFileRequirements::default()
    };
    let conflicts = JsonFileRequirements::merge(vec![
        (provenance("scalar-policy"), scalar),
        (provenance("list-policy"), list),
    ])
    .expect_err("value kinds must conflict");
    assert!(conflicts.iter().any(|conflict| {
        conflict.reason == "json-value-kind-disagree" && conflict.contributors.len() == 2
    }));
}

#[test]
fn scalar_equals_and_oneof_use_core_resolution() {
    let path = JsonPath::new("mode");
    let equals = JsonFileRequirements {
        scalar_values: BTreeMap::from([(
            path.clone(),
            ScalarAssertion::Equals(ConfigScalar::Str("safe".to_owned()), "equals".to_owned()),
        )]),
        ..JsonFileRequirements::default()
    };
    let one_of = JsonFileRequirements {
        scalar_values: BTreeMap::from([(
            path,
            ScalarAssertion::OneOf(
                BTreeSet::from([ConfigScalar::Str("safe".to_owned())]),
                "oneof".to_owned(),
            ),
        )]),
        ..JsonFileRequirements::default()
    };
    let resolved = JsonFileRequirements::merge(vec![
        (provenance("equals-policy"), equals),
        (provenance("oneof-policy"), one_of),
    ])
    .expect("compatible scalar assertions must resolve");
    assert_eq!(resolved.scalar_values().len(), 1);
}

#[test]
fn collection_merge_conflicts_use_json_pointer_member_keys() {
    let list_path = JsonPath::new("items");
    let object_path = JsonPath::new("object");
    let first = JsonFileRequirements {
        string_lists: BTreeMap::from([(
            list_path.clone(),
            ListRequirements {
                contains: BTreeMap::from([("a/b".to_owned(), "required".to_owned())]),
                ..ListRequirements::default()
            },
        )]),
        object_keys: BTreeMap::from([(
            object_path.clone(),
            ItemRequirements {
                required: vec![(
                    KeyedItem {
                        file_key: "a/b".to_owned(),
                        value: (),
                    },
                    "required".to_owned(),
                )],
                ..ItemRequirements::default()
            },
        )]),
        ..JsonFileRequirements::default()
    };
    let second = JsonFileRequirements {
        string_lists: BTreeMap::from([(
            list_path,
            ListRequirements {
                excludes: BTreeMap::from([("a/b".to_owned(), "forbidden".to_owned())]),
                ..ListRequirements::default()
            },
        )]),
        object_keys: BTreeMap::from([(
            object_path,
            ItemRequirements {
                forbidden: vec![(
                    KeyedItem {
                        file_key: "a/b".to_owned(),
                        value: (),
                    },
                    "forbidden".to_owned(),
                )],
                ..ItemRequirements::default()
            },
        )]),
        ..JsonFileRequirements::default()
    };
    let conflicts = JsonFileRequirements::merge(vec![
        (provenance("first"), first),
        (provenance("second"), second),
    ])
    .expect_err("collection requirements must conflict");

    assert!(conflicts.iter().any(|conflict| {
        conflict.reason == "list-contains-and-excludes" && conflict.key == "/items/a~1b"
    }));
    assert!(conflicts.iter().any(|conflict| {
        conflict.reason == "item-required-and-forbidden" && conflict.key == "/object/a~1b"
    }));
}

#[test]
fn strict_duplicate_non_object_non_string_and_invalid_utf8_inputs_fail_closed() {
    let requirement = JsonFileRequirements {
        string_lists: BTreeMap::from([(
            JsonPath::new("items"),
            ListRequirements {
                contains: BTreeMap::from([("required".to_owned(), "message".to_owned())]),
                ..ListRequirements::default()
            },
        )]),
        ..JsonFileRequirements::default()
    };
    let resolved = JsonFileRequirements::merge(vec![(provenance("policy"), requirement)])
        .expect("requirements must resolve");
    for input in [
        br#"{"items":[],"items":[]}"#.as_slice(),
        br"[]".as_slice(),
        b"{\"items\":[\"ok\",1]}".as_slice(),
        b"{\"items\":\"wrong\"}".as_slice(),
        b"{\"items\":\"\xff\"}".as_slice(),
    ] {
        let output = JsonFileEngine::reconcile(Some(input), &resolved);
        assert!(!output.findings.is_empty(), "input must fail: {input:?}");
    }
}

#[test]
fn missing_document_generation_is_idempotent_and_preserves_json_pointer_identity() {
    let path = JsonPath::new("a/b").child("~value");
    assert_eq!(path.pointer(), "/a~1b/~0value");
    let requirement = JsonFileRequirements {
        scalar_values: BTreeMap::from([(
            path,
            ScalarAssertion::Equals(ConfigScalar::Int(2), "value".to_owned()),
        )]),
        ..JsonFileRequirements::default()
    };
    let resolved = JsonFileRequirements::merge(vec![(provenance("policy"), requirement)])
        .expect("requirements must resolve");
    let first = JsonFileEngine::reconcile(None, &resolved);
    assert!(format!("{:?}", first.findings).contains("/a~1b/~0value"));
    let second = JsonFileEngine::reconcile(Some(&first.expected_bytes), &resolved);
    assert!(second.findings.is_empty());
    assert_eq!(first.expected_bytes, second.expected_bytes);
}

#[test]
fn exact_root_keys_report_and_remove_only_extra_keys() {
    let requirement = JsonFileRequirements {
        scalar_values: BTreeMap::from([(
            JsonPath::new("managed"),
            ScalarAssertion::Equals(ConfigScalar::Bool(true), "managed message".to_owned()),
        )]),
        object_keys: BTreeMap::from([(
            JsonPath::root(),
            ItemRequirements {
                allowed: None,
                exact: Some((
                    vec![KeyedItem {
                        file_key: "managed".to_owned(),
                        value: (),
                    }],
                    "exact root message".to_owned(),
                )),
                ..ItemRequirements::default()
            },
        )]),
        ..JsonFileRequirements::default()
    };
    let resolved = JsonFileRequirements::merge(vec![(provenance("policy"), requirement)])
        .expect("requirements must resolve");
    let output = JsonFileEngine::reconcile(Some(br#"{"managed":true,"extra":1}"#), &resolved);
    let finding = format!("{:?}", output.findings);
    assert!(finding.contains("key: \"$\""));
    assert!(finding.contains("extra"));
    assert!(finding.contains("exact root message"));
    assert_eq!(output.expected_bytes, b"{\"managed\":true}");
}

#[test]
fn exact_root_keys_accept_a_matching_root_without_a_shape_finding() {
    let requirement = JsonFileRequirements {
        object_keys: BTreeMap::from([(
            JsonPath::root(),
            ItemRequirements {
                allowed: None,
                exact: Some((
                    vec![KeyedItem {
                        file_key: "managed".to_owned(),
                        value: (),
                    }],
                    "exact root".to_owned(),
                )),
                ..ItemRequirements::default()
            },
        )]),
        ..JsonFileRequirements::default()
    };
    let resolved = JsonFileRequirements::merge(vec![(provenance("policy"), requirement)])
        .expect("requirements must resolve");
    let input = br#"{"managed":true}"#;
    let output = JsonFileEngine::reconcile(Some(input), &resolved);
    assert!(output.findings.is_empty());
    assert_eq!(output.expected_bytes, input);
}

#[test]
fn object_closure_cannot_exclude_a_managed_descendant() {
    let requirement = JsonFileRequirements {
        scalar_values: BTreeMap::from([(
            JsonPath::new("parent").child("child"),
            ScalarAssertion::Equals(ConfigScalar::Bool(true), "child".to_owned()),
        )]),
        object_keys: BTreeMap::from([(
            JsonPath::new("parent"),
            ItemRequirements {
                allowed: None,
                exact: Some((Vec::new(), "closed parent".to_owned())),
                ..ItemRequirements::default()
            },
        )]),
        ..JsonFileRequirements::default()
    };
    let conflicts = JsonFileRequirements::merge(vec![(provenance("policy"), requirement)])
        .expect_err("closure excluding a managed child must conflict");
    assert!(conflicts.iter().any(|conflict| {
        conflict.reason == "json-object-closure-excludes-managed-descendant"
            && conflict.key == "/parent/child"
    }));
}

#[test]
fn object_membership_conflicts_only_with_descendants_that_require_presence() {
    let absent_child = JsonFileRequirements {
        scalar_values: BTreeMap::from([(
            JsonPath::new("parent").child("child"),
            ScalarAssertion::Absent("child absent".to_owned()),
        )]),
        ..JsonFileRequirements::default()
    };
    let forbidden_child = JsonFileRequirements {
        object_keys: BTreeMap::from([(
            JsonPath::new("parent"),
            ItemRequirements {
                forbidden: vec![(
                    KeyedItem {
                        file_key: "child".to_owned(),
                        value: (),
                    },
                    "child forbidden".to_owned(),
                )],
                ..ItemRequirements::default()
            },
        )]),
        ..JsonFileRequirements::default()
    };
    let _ = JsonFileRequirements::merge(vec![
        (provenance("absent"), absent_child.clone()),
        (provenance("forbidden"), forbidden_child),
    ])
    .expect("two absence requirements are compatible");

    let required_child = JsonFileRequirements {
        object_keys: BTreeMap::from([(
            JsonPath::new("parent"),
            ItemRequirements {
                required: vec![(
                    KeyedItem {
                        file_key: "child".to_owned(),
                        value: (),
                    },
                    "child required".to_owned(),
                )],
                ..ItemRequirements::default()
            },
        )]),
        ..JsonFileRequirements::default()
    };
    let conflicts = JsonFileRequirements::merge(vec![
        (provenance("absent"), absent_child),
        (provenance("required"), required_child),
    ])
    .expect_err("required and absent child requirements must conflict");
    assert!(conflicts.iter().any(|conflict| {
        conflict.reason == "json-value-required-and-forbidden" && conflict.key == "/parent/child"
    }));
}

#[test]
fn same_surface_presence_conflicts_are_reported_only_by_core() {
    let path = JsonPath::new("value");
    let absent = JsonFileRequirements {
        scalar_values: BTreeMap::from([(
            path.clone(),
            ScalarAssertion::Absent("absent".to_owned()),
        )]),
        ..JsonFileRequirements::default()
    };
    let present = JsonFileRequirements {
        scalar_values: BTreeMap::from([(path, ScalarAssertion::Present("present".to_owned()))]),
        ..JsonFileRequirements::default()
    };
    let conflicts = JsonFileRequirements::merge(vec![
        (provenance("absent"), absent),
        (provenance("present"), present),
    ])
    .expect_err("absent and present scalar requirements must conflict");
    assert_eq!(conflicts.len(), 1);
    assert_eq!(
        conflicts.first().expect("one conflict").reason,
        "scalar-disagree"
    );
}

#[test]
fn same_surface_presence_conflict_does_not_hide_kind_conflict() {
    let path = JsonPath::new("value");
    let scalar = JsonFileRequirements {
        scalar_values: BTreeMap::from([(
            path.clone(),
            ScalarAssertion::Present("present".to_owned()),
        )]),
        ..JsonFileRequirements::default()
    };
    let absent = JsonFileRequirements {
        scalar_values: BTreeMap::from([(
            path.clone(),
            ScalarAssertion::Absent("absent".to_owned()),
        )]),
        ..JsonFileRequirements::default()
    };
    let list = JsonFileRequirements {
        string_lists: BTreeMap::from([(
            path,
            ListRequirements {
                contains: BTreeMap::from([("required".to_owned(), "required".to_owned())]),
                ..ListRequirements::default()
            },
        )]),
        ..JsonFileRequirements::default()
    };
    let conflicts = JsonFileRequirements::merge(vec![
        (provenance("policy"), scalar),
        (provenance("policy"), absent),
        (provenance("policy"), list),
    ])
    .expect_err("presence and kind requirements must both conflict");
    assert!(
        conflicts
            .iter()
            .any(|conflict| conflict.reason == "scalar-disagree")
    );
    assert!(
        conflicts
            .iter()
            .any(|conflict| conflict.reason == "json-value-kind-disagree")
    );
}

#[test]
fn nonconstructive_requirement_does_not_duplicate_presence_conflict() {
    let path = JsonPath::new("value");
    let absent = JsonFileRequirements {
        scalar_values: BTreeMap::from([(
            path.clone(),
            ScalarAssertion::Absent("absent".to_owned()),
        )]),
        ..JsonFileRequirements::default()
    };
    let contains = JsonFileRequirements {
        string_lists: BTreeMap::from([(
            path.clone(),
            ListRequirements {
                contains: BTreeMap::from([("required".to_owned(), "required".to_owned())]),
                ..ListRequirements::default()
            },
        )]),
        ..JsonFileRequirements::default()
    };
    let excludes = JsonFileRequirements {
        string_lists: BTreeMap::from([(
            path,
            ListRequirements {
                excludes: BTreeMap::from([("blocked".to_owned(), "blocked".to_owned())]),
                ..ListRequirements::default()
            },
        )]),
        ..JsonFileRequirements::default()
    };
    let conflicts = JsonFileRequirements::merge(vec![
        (provenance("policy"), absent),
        (provenance("policy"), contains),
        (provenance("policy"), excludes),
    ])
    .expect_err("absent and constructive list requirements must conflict");
    assert_eq!(conflicts.len(), 1);
    assert_eq!(
        conflicts.first().expect("one conflict").reason,
        "json-value-required-and-forbidden"
    );
}

#[test]
fn kind_conflict_attributes_only_unexplained_kind_contributors() {
    let path = JsonPath::new("value");
    let scalar = |assertion| JsonFileRequirements {
        scalar_values: BTreeMap::from([(path.clone(), assertion)]),
        ..JsonFileRequirements::default()
    };
    let list = JsonFileRequirements {
        string_lists: BTreeMap::from([(
            path.clone(),
            ListRequirements {
                contains: BTreeMap::from([("required".to_owned(), "required".to_owned())]),
                ..ListRequirements::default()
            },
        )]),
        ..JsonFileRequirements::default()
    };
    let present = scalar(ScalarAssertion::Present("present".to_owned()));
    let absent = scalar(ScalarAssertion::Absent("absent".to_owned()));
    let conflicts = JsonFileRequirements::merge(vec![
        (provenance("present"), present.clone()),
        (provenance("absent"), absent.clone()),
        (provenance("list"), list.clone()),
    ])
    .expect_err("presence and kind requirements must conflict");
    let kind = conflicts
        .iter()
        .find(|conflict| conflict.reason == "json-value-kind-disagree")
        .expect("merge must report the unexplained JSON value-kind conflict");
    let policies = kind
        .contributors
        .iter()
        .map(|(provenance, _)| provenance.policy.as_str())
        .collect::<Vec<_>>();
    assert_eq!(policies, ["list", "present"]);
    let reversed = JsonFileRequirements::merge(vec![
        (provenance("list"), list),
        (provenance("absent"), absent),
        (provenance("present"), present),
    ])
    .expect_err("reordered requirements must conflict");
    let reversed_kind = reversed
        .iter()
        .find(|conflict| conflict.reason == "json-value-kind-disagree")
        .expect("reordered kind conflict");
    assert_eq!(kind.contributors, reversed_kind.contributors);
}

#[test]
fn same_object_membership_conflicts_are_reported_only_by_core() {
    let item = KeyedItem {
        file_key: "child".to_owned(),
        value: (),
    };
    let requirement = JsonFileRequirements {
        object_keys: BTreeMap::from([(
            JsonPath::new("parent"),
            ItemRequirements {
                required: vec![(item.clone(), "required".to_owned())],
                forbidden: vec![(item, "forbidden".to_owned())],
                ..ItemRequirements::default()
            },
        )]),
        ..JsonFileRequirements::default()
    };
    let conflicts = JsonFileRequirements::merge(vec![(provenance("policy"), requirement)])
        .expect_err("required and forbidden object membership must conflict");
    assert_eq!(conflicts.len(), 1);
    assert_eq!(
        conflicts.first().expect("one conflict").reason,
        "item-required-and-forbidden"
    );
}

#[test]
fn object_closure_accepts_descendants_that_do_not_require_presence() {
    let nonconstructive_list = JsonFileRequirements {
        string_lists: BTreeMap::from([(
            JsonPath::new("parent").child("child"),
            ListRequirements {
                excludes: BTreeMap::from([("blocked".to_owned(), "blocked".to_owned())]),
                ..ListRequirements::default()
            },
        )]),
        ..JsonFileRequirements::default()
    };
    let exact_without_child = JsonFileRequirements {
        object_keys: BTreeMap::from([(
            JsonPath::new("parent"),
            ItemRequirements {
                allowed: None,
                exact: Some((Vec::new(), "empty parent".to_owned())),
                ..ItemRequirements::default()
            },
        )]),
        ..JsonFileRequirements::default()
    };
    let _ = JsonFileRequirements::merge(vec![
        (provenance("list"), nonconstructive_list),
        (provenance("exact"), exact_without_child),
    ])
    .expect("closure and nonconstructive descendant requirements are compatible");
}

#[test]
fn required_glob_conflict_is_unique_complete_and_member_keyed() {
    let path = JsonPath::new("items");
    let required = JsonFileRequirements {
        string_lists: BTreeMap::from([(
            path.clone(),
            ListRequirements {
                contains: BTreeMap::from([("a/b.ts".to_owned(), "contains".to_owned())]),
                exact: Some((vec!["a/b.ts".to_owned()], "exact".to_owned())),
                ..ListRequirements::default()
            },
        )]),
        ..JsonFileRequirements::default()
    };
    let glob = |value: &str| JsonFileRequirements {
        forbidden_string_list_globs: BTreeMap::from([(
            path.clone(),
            ForbiddenGlobRequirements {
                globs: vec![(
                    JsonStringGlob {
                        glob: value.to_owned(),
                    },
                    value.to_owned(),
                )],
            },
        )]),
        ..JsonFileRequirements::default()
    };
    let conflicts = JsonFileRequirements::merge(vec![
        (provenance("required"), required),
        (provenance("typescript"), glob("**/*.ts")),
        (provenance("nested"), glob("a/**")),
    ])
    .expect_err("required member matched by forbidden globs must conflict");
    let matching = conflicts
        .iter()
        .filter(|conflict| conflict.reason == "list-required-and-forbidden-glob")
        .collect::<Vec<_>>();
    assert_eq!(matching.len(), 1);
    let conflict = matching.first().expect("one matching conflict");
    assert_eq!(conflict.key, "/items/a~1b.ts");
    assert_eq!(conflict.contributors.len(), 3);
}

#[test]
fn invalid_glob_prevents_every_other_edit() {
    let requirement = JsonFileRequirements {
        scalar_values: BTreeMap::from([(
            JsonPath::new("enabled"),
            ScalarAssertion::Equals(ConfigScalar::Bool(true), "enabled".to_owned()),
        )]),
        forbidden_string_list_globs: BTreeMap::from([(
            JsonPath::new("items"),
            ForbiddenGlobRequirements {
                globs: vec![(
                    JsonStringGlob {
                        glob: "[".to_owned(),
                    },
                    "invalid glob".to_owned(),
                )],
            },
        )]),
        ..JsonFileRequirements::default()
    };
    let resolved = JsonFileRequirements::merge(vec![(provenance("policy"), requirement)])
        .expect("glob syntax is diagnosed during reconciliation");
    let input = br#"{"enabled":false,"items":[]}"#;
    let output = JsonFileEngine::reconcile(Some(input), &resolved);
    assert_eq!(output.expected_bytes, input);
    assert!(format!("{:?}", output.findings).contains("InvalidRequirements"));
}

#[test]
fn exact_empty_nested_object_is_initialized() {
    let requirement = JsonFileRequirements {
        object_keys: BTreeMap::from([(
            JsonPath::new("nested"),
            ItemRequirements {
                allowed: None,
                exact: Some((Vec::new(), "empty object".to_owned())),
                ..ItemRequirements::default()
            },
        )]),
        ..JsonFileRequirements::default()
    };
    let resolved = JsonFileRequirements::merge(vec![(provenance("policy"), requirement)])
        .expect("requirements must resolve");
    let output = JsonFileEngine::reconcile(Some(b"{}"), &resolved);
    assert_eq!(output.findings.len(), 1);
    assert_eq!(output.expected_bytes, b"{\n  \"nested\": {}\n}");
}

#[test]
fn allowed_only_object_keys_remove_extras_without_creating_optional_keys() {
    let requirement = JsonFileRequirements {
        object_keys: BTreeMap::from([(
            JsonPath::root(),
            ItemRequirements {
                allowed: Some((
                    vec![KeyedItem {
                        file_key: "optional".to_owned(),
                        value: (),
                    }],
                    "only optional is allowed".to_owned(),
                )),
                ..ItemRequirements::default()
            },
        )]),
        ..JsonFileRequirements::default()
    };
    let resolved = JsonFileRequirements::merge(vec![(provenance("policy"), requirement)])
        .expect("requirements must resolve");
    let output = JsonFileEngine::reconcile(Some(br#"{"extra":1}"#), &resolved);

    assert_eq!(output.expected_bytes, b"{}");
    assert_eq!(output.findings.len(), 1);
}

#[test]
fn descendant_objects_are_created_before_ancestor_key_checks() {
    let requirement = JsonFileRequirements {
        object_keys: BTreeMap::from([
            (
                JsonPath::new("parent"),
                ItemRequirements {
                    allowed: None,
                    exact: Some((
                        vec![KeyedItem {
                            file_key: "child".to_owned(),
                            value: (),
                        }],
                        "parent keys".to_owned(),
                    )),
                    ..ItemRequirements::default()
                },
            ),
            (
                JsonPath::new("parent").child("child"),
                ItemRequirements {
                    allowed: None,
                    exact: Some((Vec::new(), "child object".to_owned())),
                    ..ItemRequirements::default()
                },
            ),
        ]),
        ..JsonFileRequirements::default()
    };
    let resolved = JsonFileRequirements::merge(vec![(provenance("policy"), requirement)])
        .expect("requirements must resolve");
    let output = JsonFileEngine::reconcile(Some(b"{}"), &resolved);
    assert!(!format!("{:?}", output.findings).contains("UnwritableRequiredKey"));
    let second = JsonFileEngine::reconcile(Some(&output.expected_bytes), &resolved);
    assert!(second.findings.is_empty());
}

#[test]
fn overlapping_globs_combine_attribution_and_duplicate_values_report_once() {
    let path = JsonPath::new("items");
    let first = JsonFileRequirements {
        forbidden_string_list_globs: BTreeMap::from([(
            path.clone(),
            ForbiddenGlobRequirements {
                globs: vec![(
                    JsonStringGlob {
                        glob: "*.ts".to_owned(),
                    },
                    "typescript".to_owned(),
                )],
            },
        )]),
        ..JsonFileRequirements::default()
    };
    let second = JsonFileRequirements {
        forbidden_string_list_globs: BTreeMap::from([(
            path,
            ForbiddenGlobRequirements {
                globs: vec![(
                    JsonStringGlob {
                        glob: "generated.*".to_owned(),
                    },
                    "generated".to_owned(),
                )],
            },
        )]),
        ..JsonFileRequirements::default()
    };
    let resolved = JsonFileRequirements::merge(vec![
        (provenance("first"), first),
        (provenance("second"), second),
    ])
    .expect("requirements must resolve");
    let output = JsonFileEngine::reconcile(
        Some(br#"{"items":["generated.ts","generated.ts"]}"#),
        &resolved,
    );
    assert_eq!(output.findings.len(), 1);
    let finding = format!("{:?}", output.findings);
    assert!(finding.contains("first"));
    assert!(finding.contains("second"));
}

#[test]
fn root_leaf_requirements_are_rejected() {
    let requirement = JsonFileRequirements {
        scalar_values: BTreeMap::from([(
            JsonPath::root(),
            ScalarAssertion::Equals(ConfigScalar::Bool(true), "root scalar".to_owned()),
        )]),
        ..JsonFileRequirements::default()
    };
    let conflicts = JsonFileRequirements::merge(vec![(provenance("policy"), requirement)])
        .expect_err("the strict JSON engine requires an object root");
    assert!(
        conflicts
            .iter()
            .any(|conflict| conflict.reason == "json-root-must-be-object")
    );
}

#[test]
fn list_creation_distinguishes_exact_empty_from_non_constructive_requirements() {
    let path = JsonPath::new("items");
    let exact = JsonFileRequirements {
        string_lists: BTreeMap::from([(
            path.clone(),
            ListRequirements {
                exact: Some((Vec::new(), "empty".to_owned())),
                ..ListRequirements::default()
            },
        )]),
        ..JsonFileRequirements::default()
    };
    let resolved = JsonFileRequirements::merge(vec![(provenance("policy"), exact)])
        .expect("requirements must resolve");
    let output = JsonFileEngine::reconcile(Some(b"{}"), &resolved);
    assert_eq!(output.findings.len(), 1);
    assert!(
        String::from_utf8(output.expected_bytes)
            .expect("JSON is UTF-8")
            .contains("\"items\": []")
    );

    let non_constructive = JsonFileRequirements {
        string_lists: BTreeMap::from([(
            path.clone(),
            ListRequirements {
                excludes: BTreeMap::from([("blocked".to_owned(), "exclude".to_owned())]),
                ..ListRequirements::default()
            },
        )]),
        forbidden_string_list_globs: BTreeMap::from([(
            path,
            ForbiddenGlobRequirements {
                globs: vec![(
                    JsonStringGlob {
                        glob: "*.generated".to_owned(),
                    },
                    "glob".to_owned(),
                )],
            },
        )]),
        ..JsonFileRequirements::default()
    };
    let non_constructive_resolved =
        JsonFileRequirements::merge(vec![(provenance("policy"), non_constructive)])
            .expect("requirements must resolve");
    assert_eq!(
        JsonFileEngine::reconcile(Some(b"{}"), &non_constructive_resolved).expected_bytes,
        b"{}"
    );
}

#[test]
fn empty_collection_requirements_are_no_ops() {
    let path = JsonPath::new("value");
    let requirement = JsonFileRequirements {
        string_lists: BTreeMap::from([(path.clone(), ListRequirements::default())]),
        forbidden_string_list_globs: BTreeMap::from([(
            path.clone(),
            ForbiddenGlobRequirements::default(),
        )]),
        object_keys: BTreeMap::from([(path, ItemRequirements::default())]),
        ..JsonFileRequirements::default()
    };
    let resolved = JsonFileRequirements::merge(vec![(provenance("policy"), requirement)])
        .expect("empty requirements must not conflict");
    let input = br#"{"value":false}"#;
    let output = JsonFileEngine::reconcile(Some(input), &resolved);
    assert!(output.findings.is_empty());
    assert_eq!(output.expected_bytes, input);
}

#[test]
fn exact_and_glob_findings_keep_member_selectors_including_empty_values() {
    let path = JsonPath::new("items");
    let requirement = JsonFileRequirements {
        string_lists: BTreeMap::from([(
            path.clone(),
            ListRequirements {
                exact: Some((vec!["required".to_owned()], "exact list".to_owned())),
                ..ListRequirements::default()
            },
        )]),
        forbidden_string_list_globs: BTreeMap::from([(
            path,
            ForbiddenGlobRequirements {
                globs: vec![(
                    JsonStringGlob {
                        glob: String::new(),
                    },
                    "empty member".to_owned(),
                )],
            },
        )]),
        ..JsonFileRequirements::default()
    };
    let resolved = JsonFileRequirements::merge(vec![(provenance("policy"), requirement)])
        .expect("requirements must resolve");
    let output = JsonFileEngine::reconcile(Some(br#"{"items":[""]}"#), &resolved);
    assert!(output.findings.iter().any(|finding| matches!(
        finding,
        Finding::Mismatch {
            selector: Some(selector),
            ..
        } if selector == "required"
    )));
    assert!(output.findings.iter().any(|finding| matches!(
        finding,
        Finding::Mismatch {
            selector: Some(selector),
            ..
        } if selector.is_empty()
    )));
}

#[test]
fn exact_list_findings_are_member_specific_order_aware_and_constructive() {
    let path = JsonPath::new("items");
    let requirement = JsonFileRequirements {
        string_lists: BTreeMap::from([(
            path,
            ListRequirements {
                exact: Some((
                    vec!["a".to_owned(), "a".to_owned(), "b".to_owned()],
                    "exact list".to_owned(),
                )),
                ..ListRequirements::default()
            },
        )]),
        ..JsonFileRequirements::default()
    };
    let resolved = JsonFileRequirements::merge(vec![(provenance("policy"), requirement)])
        .expect("requirements must resolve");

    let membership = JsonFileEngine::reconcile(Some(br#"{"items":["a","c"]}"#), &resolved);
    assert_eq!(membership.findings.len(), 3);
    let selectors = membership
        .findings
        .iter()
        .filter_map(|finding| match finding {
            Finding::Mismatch { selector, .. } => selector.clone(),
            Finding::UnwritableRequiredKey { .. }
            | Finding::InvalidRequirements { .. }
            | Finding::ParseError { .. }
            | Finding::ConflictingRequirements { .. }
            | Finding::InternalError { .. } => None,
        })
        .collect::<BTreeSet<_>>();
    assert_eq!(
        selectors,
        BTreeSet::from(["a".to_owned(), "b".to_owned(), "c".to_owned()])
    );
    assert!(membership.findings.iter().all(|finding| matches!(
        finding,
        Finding::Mismatch {
            attribution,
            ..
        } if attribution == &vec![provenance("policy")]
    )));

    let order = JsonFileEngine::reconcile(Some(br#"{"items":["b","a","a"]}"#), &resolved);
    assert!(matches!(
        order.findings.as_slice(),
        [Finding::Mismatch { selector: None, .. }]
    ));

    let missing = JsonFileEngine::reconcile(Some(br"{}"), &resolved);
    assert!(matches!(
        missing.findings.as_slice(),
        [Finding::Mismatch {
            selector: None,
            current: None,
            ..
        }]
    ));
    assert!(
        String::from_utf8(missing.expected_bytes)
            .expect("JSON is UTF-8")
            .contains(r#""items": ["#)
    );
}

#[test]
fn compatible_exact_member_assertions_share_json_selector_identity() {
    let exact = JsonFileRequirements {
        string_lists: BTreeMap::from([(
            JsonPath::new("items"),
            ListRequirements {
                exact: Some((vec!["react".to_owned()], "exact".to_owned())),
                ..ListRequirements::default()
            },
        )]),
        ..JsonFileRequirements::default()
    };
    let contains = JsonFileRequirements {
        string_lists: BTreeMap::from([(
            JsonPath::new("items"),
            ListRequirements {
                contains: BTreeMap::from([("react".to_owned(), "contains".to_owned())]),
                ..ListRequirements::default()
            },
        )]),
        ..JsonFileRequirements::default()
    };
    let excludes = JsonFileRequirements {
        string_lists: BTreeMap::from([(
            JsonPath::new("items"),
            ListRequirements {
                excludes: BTreeMap::from([("blocked".to_owned(), "excludes".to_owned())]),
                ..ListRequirements::default()
            },
        )]),
        ..JsonFileRequirements::default()
    };
    let resolved = JsonFileRequirements::merge(vec![
        (provenance("exact-policy"), exact),
        (provenance("contains-policy"), contains),
        (provenance("excludes-policy"), excludes),
    ])
    .expect("compatible requirements must resolve");

    for (input, selector, expected) in [
        (
            br#"{"items":[]}"#.as_slice(),
            "react",
            [("exact", "exact-policy"), ("contains", "contains-policy")],
        ),
        (
            br#"{"items":["react","blocked"]}"#.as_slice(),
            "blocked",
            [("exact", "exact-policy"), ("excludes", "excludes-policy")],
        ),
    ] {
        let output = JsonFileEngine::reconcile(Some(input), &resolved);
        assert_eq!(output.findings.len(), 2);
        for (message, policy) in expected {
            assert!(output.findings.iter().any(|finding| matches!(
                finding,
                Finding::Mismatch { key, selector: Some(found), message: found_message, attribution, .. }
                    if key == "/items"
                        && found == selector
                        && found_message == message
                        && attribution == &vec![provenance(policy)]
            )));
        }
    }
}

#[test]
fn malformed_list_and_blocked_parent_are_reported_without_rewriting_bytes() {
    let requirement = JsonFileRequirements {
        string_lists: BTreeMap::from([(
            JsonPath::new("parent").child("items"),
            ListRequirements {
                contains: BTreeMap::from([("required".to_owned(), "message".to_owned())]),
                ..ListRequirements::default()
            },
        )]),
        ..JsonFileRequirements::default()
    };
    let resolved = JsonFileRequirements::merge(vec![(provenance("policy"), requirement)])
        .expect("requirements must resolve");
    for input in [
        br#"{"parent":false}"#.as_slice(),
        br#"{"parent":{"items":[1]}}"#,
    ] {
        let output = JsonFileEngine::reconcile(Some(input), &resolved);
        assert!(!output.findings.is_empty());
        assert_eq!(output.expected_bytes, input);
    }
}

#[test]
fn blocked_object_parent_reports_one_shape_finding_without_rewriting_bytes() {
    let requirement = JsonFileRequirements {
        object_keys: BTreeMap::from([(
            JsonPath::new("parent").child("nested"),
            ItemRequirements {
                allowed: None,
                exact: Some((Vec::new(), "empty object".to_owned())),
                ..ItemRequirements::default()
            },
        )]),
        ..JsonFileRequirements::default()
    };
    let resolved = JsonFileRequirements::merge(vec![(provenance("policy"), requirement)])
        .expect("requirements must resolve");
    let input = br#"{"parent":false}"#;
    let output = JsonFileEngine::reconcile(Some(input), &resolved);

    assert_eq!(output.findings.len(), 1);
    assert_eq!(output.expected_bytes, input);
}

#[test]
fn invalid_glob_fails_closed_with_policy_attribution() {
    let requirement = JsonFileRequirements {
        forbidden_string_list_globs: BTreeMap::from([(
            JsonPath::new("items"),
            ForbiddenGlobRequirements {
                globs: vec![(
                    JsonStringGlob {
                        glob: "[".to_owned(),
                    },
                    "invalid glob source".to_owned(),
                )],
            },
        )]),
        ..JsonFileRequirements::default()
    };
    let resolved = JsonFileRequirements::merge(vec![(provenance("policy"), requirement)])
        .expect("glob syntax is diagnosed during reconciliation");
    let output = JsonFileEngine::reconcile(Some(br#"{"items":["value"]}"#), &resolved);
    let finding = format!("{:?}", output.findings);
    assert!(finding.contains("InvalidRequirements"));
    assert!(finding.contains("invalid glob source"));
    assert_eq!(output.expected_bytes, br#"{"items":["value"]}"#);
}

#[test]
fn merge_rejects_leaf_descendants_and_required_items_forbidden_by_glob() {
    let leaf = JsonFileRequirements {
        scalar_values: BTreeMap::from([(
            JsonPath::new("parent"),
            ScalarAssertion::Equals(ConfigScalar::Bool(true), "leaf".to_owned()),
        )]),
        ..JsonFileRequirements::default()
    };
    let descendant = JsonFileRequirements {
        scalar_values: BTreeMap::from([(
            JsonPath::new("parent").child("child"),
            ScalarAssertion::Equals(ConfigScalar::Bool(true), "child".to_owned()),
        )]),
        ..JsonFileRequirements::default()
    };
    let conflicts = JsonFileRequirements::merge(vec![
        (provenance("leaf"), leaf),
        (provenance("descendant"), descendant),
    ])
    .expect_err("leaf and descendant requirements must conflict");
    assert!(
        conflicts
            .iter()
            .any(|conflict| conflict.reason == "json-leaf-has-required-descendant")
    );

    let required_and_forbidden = JsonFileRequirements {
        string_lists: BTreeMap::from([(
            JsonPath::new("items"),
            ListRequirements {
                contains: BTreeMap::from([("generated.ts".to_owned(), "required".to_owned())]),
                ..ListRequirements::default()
            },
        )]),
        forbidden_string_list_globs: BTreeMap::from([(
            JsonPath::new("items"),
            ForbiddenGlobRequirements {
                globs: vec![(
                    JsonStringGlob {
                        glob: "*.ts".to_owned(),
                    },
                    "forbidden".to_owned(),
                )],
            },
        )]),
        ..JsonFileRequirements::default()
    };
    let required_glob_conflicts =
        JsonFileRequirements::merge(vec![(provenance("policy"), required_and_forbidden)])
            .expect_err("required item forbidden by glob must conflict");
    assert!(
        required_glob_conflicts
            .iter()
            .any(|conflict| conflict.reason == "list-required-and-forbidden-glob")
    );
}
