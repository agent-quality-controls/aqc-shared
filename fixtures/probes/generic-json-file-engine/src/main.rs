use std::collections::BTreeMap;
use std::path::Path;

use aqc_file_engine_core::{FileEngine as _, Finding};
use aqc_json_file_engine::{
    ConfigScalar, ForbiddenGlobRequirements, ItemRequirements, JsonFileEngine,
    JsonFileRequirements, JsonPath, JsonStringGlob, KeyedItem, ListRequirements, Provenance,
    ScalarAssertion,
};
use serde_json::{Value, json};

fn provenance() -> Provenance {
    Provenance {
        policy: "fixture-policy".to_owned(),
    }
}

fn reconcile(
    input: &[u8],
    requirement: JsonFileRequirements,
) -> aqc_file_engine_core::EngineOutput {
    let resolved = JsonFileRequirements::merge(vec![(provenance(), requirement)])
        .expect("fixture requirement must resolve");
    JsonFileEngine::reconcile(Some(input), &resolved)
}

fn finding(value: &Finding) -> Value {
    match value {
        Finding::Mismatch {
            key,
            selector,
            expected,
            message,
            ..
        } => json!({
            "kind": "mismatch",
            "key": key,
            "selector": selector,
            "expected": expected,
            "message": message,
        }),
        Finding::InvalidRequirements {
            key, contributors, ..
        } => {
            json!({"kind": "invalid", "key": key, "contributors": contributors})
        }
        Finding::UnwritableRequiredKey { key, .. } => json!({"kind": "unwritable", "key": key}),
        other => json!({"kind": format!("{other:?}")}),
    }
}

fn exact_empty_list() -> Value {
    let requirement = JsonFileRequirements {
        string_lists: BTreeMap::from([(
            JsonPath::new("items"),
            ListRequirements {
                exact: Some((Vec::new(), "empty list".to_owned())),
                ..ListRequirements::default()
            },
        )]),
        ..JsonFileRequirements::default()
    };
    let output = reconcile(b"{}", requirement.clone());
    let second = reconcile(&output.expected_bytes, requirement);
    json!({
        "expected": String::from_utf8(output.expected_bytes).expect("JSON is UTF-8"),
        "findings": output.findings.iter().map(finding).collect::<Vec<_>>(),
        "secondFindings": second.findings.len(),
    })
}

fn exact_empty_object() -> Value {
    let requirement = JsonFileRequirements {
        object_keys: BTreeMap::from([(
            JsonPath::new("nested"),
            ItemRequirements {
                exact: Some((Vec::new(), "empty object".to_owned())),
                ..ItemRequirements::default()
            },
        )]),
        ..JsonFileRequirements::default()
    };
    let output = reconcile(b"{}", requirement.clone());
    let second = reconcile(&output.expected_bytes, requirement);
    json!({
        "expected": String::from_utf8(output.expected_bytes).expect("JSON is UTF-8"),
        "findings": output.findings.iter().map(finding).collect::<Vec<_>>(),
        "secondFindings": second.findings.len(),
    })
}

fn descendant_object_order() -> Value {
    let requirement = JsonFileRequirements {
        object_keys: BTreeMap::from([
            (
                JsonPath::new("parent"),
                ItemRequirements {
                    exact: Some((
                        vec![KeyedItem {
                            file_key: "child".to_owned(),
                            value: (),
                        }],
                        "parent".to_owned(),
                    )),
                    ..ItemRequirements::default()
                },
            ),
            (
                JsonPath::new("parent").child("child"),
                ItemRequirements {
                    exact: Some((Vec::new(), "child".to_owned())),
                    ..ItemRequirements::default()
                },
            ),
        ]),
        ..JsonFileRequirements::default()
    };
    let output = reconcile(b"{}", requirement.clone());
    let second = reconcile(&output.expected_bytes, requirement);
    json!({
        "expected": String::from_utf8(output.expected_bytes).expect("JSON is UTF-8"),
        "findings": output.findings.iter().map(finding).collect::<Vec<_>>(),
        "secondFindings": second.findings.len(),
    })
}

fn invalid_glob_atomicity() -> Value {
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
                    "invalid".to_owned(),
                )],
            },
        )]),
        ..JsonFileRequirements::default()
    };
    let input = br#"{"enabled":false,"items":[]}"#;
    let output = reconcile(input, requirement);
    json!({
        "bytesPreserved": output.expected_bytes == input,
        "findings": output.findings.iter().map(finding).collect::<Vec<_>>(),
    })
}

fn empty_requirements() -> Value {
    let path = JsonPath::new("value");
    let output = reconcile(
        br#"{"value":false}"#,
        JsonFileRequirements {
            string_lists: BTreeMap::from([(path.clone(), ListRequirements::default())]),
            forbidden_string_list_globs: BTreeMap::from([(
                path.clone(),
                ForbiddenGlobRequirements::default(),
            )]),
            object_keys: BTreeMap::from([(path, ItemRequirements::default())]),
            ..JsonFileRequirements::default()
        },
    );
    json!({"findings": output.findings.len(), "expected": String::from_utf8(output.expected_bytes).expect("JSON is UTF-8")})
}

fn collection_selectors() -> Value {
    let path = JsonPath::new("items");
    let output = reconcile(
        br#"{"items":[""]}"#,
        JsonFileRequirements {
            string_lists: BTreeMap::from([(
                path.clone(),
                ListRequirements {
                    exact: Some((vec!["required".to_owned()], "exact".to_owned())),
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
                        "empty".to_owned(),
                    )],
                },
            )]),
            ..JsonFileRequirements::default()
        },
    );
    json!(output.findings.iter().map(finding).collect::<Vec<_>>())
}

fn finding_keys() -> Value {
    let output = reconcile(
        br#"{"a/b":{"~key":false},"extra":true}"#,
        JsonFileRequirements {
            scalar_values: BTreeMap::from([(
                JsonPath::new("a/b").child("~key"),
                ScalarAssertion::Equals(ConfigScalar::Bool(true), "nested".to_owned()),
            )]),
            object_keys: BTreeMap::from([(
                JsonPath::root(),
                ItemRequirements {
                    exact: Some((
                        vec![KeyedItem {
                            file_key: "a/b".to_owned(),
                            value: (),
                        }],
                        "root".to_owned(),
                    )),
                    ..ItemRequirements::default()
                },
            )]),
            ..JsonFileRequirements::default()
        },
    );
    json!(output.findings.iter().map(finding).collect::<Vec<_>>())
}

fn main() {
    let fixture = std::env::args().nth(1).expect("fixture path is required");
    let contract: Value =
        serde_json::from_slice(&std::fs::read(Path::new(&fixture)).expect("read fixture"))
            .expect("parse fixture");
    assert_eq!(contract["cases"].as_array().map(Vec::len), Some(7));
    println!(
        "{}",
        json!({
            "exact-empty-list": exact_empty_list(),
            "exact-empty-object": exact_empty_object(),
            "descendant-object-order": descendant_object_order(),
            "invalid-glob-atomicity": invalid_glob_atomicity(),
            "empty-requirements": empty_requirements(),
            "collection-selectors": collection_selectors(),
            "finding-keys": finding_keys(),
        })
    );
}
