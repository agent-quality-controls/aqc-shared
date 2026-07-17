#![allow(
    clippy::expect_used,
    reason = "Tests use expect to fail loudly when fixture invariants are broken."
)]
use aqc_toml_engine_core as _;
#[allow(
    dead_code,
    unused_imports,
    reason = "Shared integration test helpers; this split test uses a subset."
)]
mod common;
use common::*;

#[test]
fn dependency_required_and_forbidden_different_keys_compose() {
    let mut required = BTreeMap::new();
    let _ = required.insert(
        "serde".to_owned(),
        (dep_spec(Some("1")), "serde".to_owned()),
    );
    let mut forbidden = BTreeMap::new();
    let _ = forbidden.insert("openssl".to_owned(), "no openssl".to_owned());
    let _resolved = cargo::CargoTomlRequirements::merge(vec![(
        prov("p1"),
        dep_req(KeyedFixture {
            required,
            forbidden,
            exact: None,
        }),
    )])
    .expect("different dependency identities should merge");
}

#[test]
fn dependency_required_and_forbidden_same_key_conflicts() {
    let mut required = BTreeMap::new();
    let _ = required.insert(
        "serde".to_owned(),
        (dep_spec(Some("1")), "serde".to_owned()),
    );
    let mut forbidden = BTreeMap::new();
    let _ = forbidden.insert("serde".to_owned(), "no serde".to_owned());
    let findings = cargo_findings(vec![(
        prov("p1"),
        dep_req(KeyedFixture {
            required,
            forbidden,
            exact: None,
        }),
    )]);
    assert!(
        matches!(
            findings.first(),
            Some(engine_core::Finding::ConflictingRequirements { .. })
        ) || has_conflict(&findings)
    );
}

#[test]
fn required_and_forbidden_different_dependency_identities_coexist() {
    let req = dep_item_req(engine_core::ItemRequirements {
        required: vec![(
            package_requirement("serde_json", Some("1")),
            "serde".to_owned(),
        )],
        forbidden: vec![(
            package_requirement("openssl", None),
            "no openssl".to_owned(),
        )],
        allowed: None,
        exact: None,
    });
    let _resolved = cargo::CargoTomlRequirements::merge(vec![(prov("p1"), req)])
        .expect("different package identities should merge");
}

#[test]
fn required_and_forbidden_same_dependency_identity_conflict() {
    let findings = cargo_findings(vec![(
        prov("p1"),
        dep_item_req(engine_core::ItemRequirements {
            required: vec![(
                package_requirement("serde_json", Some("1")),
                "serde".to_owned(),
            )],
            forbidden: vec![(
                package_requirement("serde_json", None),
                "no serde".to_owned(),
            )],
            allowed: None,
            exact: None,
        }),
    )]);
    assert!(has_conflict(&findings));
}

#[test]
fn exact_collection_rejects_outside_required_identity() {
    let closer = dep_item_req(engine_core::ItemRequirements {
        required: vec![(
            package_requirement("serde_json", Some("1")),
            "serde".to_owned(),
        )],
        forbidden: Vec::new(),
        allowed: None,
        exact: Some((
            vec![package_requirement("serde_json", Some("1"))],
            "only serde".to_owned(),
        )),
    });
    let outside = dep_item_req(engine_core::ItemRequirements {
        required: vec![(package_requirement("toml", Some("0.8")), "toml".to_owned())],
        forbidden: Vec::new(),
        allowed: None,
        exact: None,
    });
    let conflicts = cargo::CargoTomlRequirements::merge(vec![
        (prov("closer"), closer),
        (prov("outside"), outside),
    ])
    .expect_err("required package outside exact set should conflict");
    assert!(conflicts.iter().any(|conflict| {
        conflict
            .contributors
            .iter()
            .any(|(p, value)| p.policy == "closer" && value == "only serde")
    }));
}

#[test]
fn exact_collection_rejects_dependency_without_identity() {
    let requirement = dep_item_req(engine_core::ItemRequirements {
        required: Vec::new(),
        forbidden: Vec::new(),
        allowed: None,
        exact: Some((
            vec![cargo::DependencyRequirement {
                file_key: None,
                value: cargo::DependencySpec::default(),
            }],
            "exact dependencies".to_owned(),
        )),
    });

    let conflicts = cargo::CargoTomlRequirements::merge(vec![(prov("policy"), requirement)])
        .expect_err("an exact dependency must have a file key or package identity");

    assert!(
        conflicts
            .iter()
            .any(|conflict| conflict.reason == "invalid-dependency-requirement")
    );
    assert!(conflicts.iter().any(|conflict| {
        conflict.contributors.iter().any(|(source, value)| {
            source.policy == "policy" && value == "missing file_key or package"
        })
    }));
}

#[test]
fn exact_collection_rejects_two_packages_using_one_file_key() {
    let requirement = dep_item_req(engine_core::ItemRequirements {
        required: Vec::new(),
        forbidden: Vec::new(),
        allowed: None,
        exact: Some((
            vec![
                local_dependency_requirement("json", Some("serde_json"), Some("1")),
                local_dependency_requirement("json", Some("json5"), Some("0.4")),
            ],
            "exact dependencies".to_owned(),
        )),
    });

    let conflicts = cargo::CargoTomlRequirements::merge(vec![(prov("policy"), requirement)])
        .expect_err("one dependency key cannot address two packages");

    assert!(
        conflicts
            .iter()
            .any(|conflict| conflict.reason == "dependency-file-key-package-conflict")
    );
    assert!(conflicts.iter().any(|conflict| {
        conflict.contributors.len() == 2
            && conflict
                .contributors
                .iter()
                .all(|(source, value)| source.policy == "policy" && value.contains("file_key json"))
    }));
}

#[test]
fn dependency_local_key_requirement_does_not_pass_on_package_only_match() {
    let req = dep_item_req(engine_core::ItemRequirements {
        required: vec![(
            local_dependency_requirement("json", Some("serde_json"), Some("1")),
            "need rename".to_owned(),
        )],
        forbidden: Vec::new(),
        allowed: None,
        exact: None,
    });
    let findings = cargo_findings_with(
        Some(b"[dependencies]\nserde_json = \"1\"\n"),
        vec![(prov("p1"), req)],
    );
    assert!(findings.iter().any(|finding| {
        matches!(finding, engine_core::Finding::Mismatch { key, .. } if key == "[dependencies].json")
    }));
}

#[test]
fn dependency_package_identity_requirement_passes_under_renamed_key() {
    let req = dep_item_req(engine_core::ItemRequirements {
        required: vec![(
            package_requirement("serde_json", Some("1")),
            "serde".to_owned(),
        )],
        forbidden: Vec::new(),
        allowed: None,
        exact: None,
    });
    let findings = cargo_findings_with(
        Some(b"[dependencies]\njson = { package = \"serde_json\", version = \"1\" }\n"),
        vec![(prov("p1"), req)],
    );
    assert!(findings.is_empty());
}

#[test]
fn dependency_package_identity_forbid_catches_renamed_key() {
    let req = dep_item_req(engine_core::ItemRequirements {
        required: Vec::new(),
        forbidden: vec![(
            package_requirement("serde_json", None),
            "no serde".to_owned(),
        )],
        allowed: None,
        exact: None,
    });
    let out = cargo_output(
        Some(b"[dependencies]\njson = { package = \"serde_json\", version = \"1\" }\n"),
        vec![(prov("p1"), req)],
    );
    let text =
        String::from_utf8(first_bytes(&out)).expect("engine output should be valid UTF-8 TOML");
    assert_eq!(out.findings.len(), 1);
    assert!(!text.contains("json ="));
}

#[test]
fn local_key_requirement_and_package_identity_forbid_conflict() {
    let findings = cargo_findings(vec![(
        prov("p1"),
        dep_item_req(engine_core::ItemRequirements {
            required: vec![(
                local_dependency_requirement("json", Some("serde_json"), Some("1")),
                "need rename".to_owned(),
            )],
            forbidden: vec![(
                package_requirement("serde_json", None),
                "no serde".to_owned(),
            )],
            allowed: None,
            exact: None,
        }),
    )]);
    assert!(has_conflict(&findings));
}

#[test]
fn same_package_identity_with_different_explicit_file_keys_conflicts() {
    let findings = cargo_findings(vec![(
        prov("p1"),
        dep_item_req(engine_core::ItemRequirements {
            required: vec![
                (
                    local_dependency_requirement("json", Some("serde_json"), Some("1")),
                    "rename".to_owned(),
                ),
                (
                    local_dependency_requirement("serde_json", Some("serde_json"), Some("1")),
                    "plain".to_owned(),
                ),
            ],
            forbidden: Vec::new(),
            allowed: None,
            exact: None,
        }),
    )]);
    assert!(findings.iter().any(|finding| {
        matches!(
            finding,
            engine_core::Finding::ConflictingRequirements { key, reason, .. }
                if key == "[dependencies].serde_json.file_key"
                    && reason == "dependency-package-multiple-file-keys"
        )
    }));
}

#[test]
fn package_identity_requirement_checks_all_duplicate_package_entries() {
    let req = dep_item_req(engine_core::ItemRequirements {
        required: vec![(
            package_requirement("serde_json", Some("1")),
            "serde".to_owned(),
        )],
        forbidden: Vec::new(),
        allowed: None,
        exact: None,
    });
    let findings = cargo_findings_with(
        Some(
            b"[dependencies]\na = { package = \"serde_json\", version = \"2\" }\nb = { package = \"serde_json\", version = \"1\" }\n",
        ),
        vec![(prov("p1"), req)],
    );
    assert!(findings.is_empty());
}

#[test]
fn local_key_json_and_package_identity_json_do_not_collide() {
    let findings = cargo_findings(vec![(
        prov("p1"),
        dep_item_req(engine_core::ItemRequirements {
            required: vec![
                (
                    local_dependency_requirement("json", Some("serde_json"), Some("1")),
                    "rename".to_owned(),
                ),
                (
                    package_requirement("json", Some("1")),
                    "json package".to_owned(),
                ),
            ],
            forbidden: Vec::new(),
            allowed: None,
            exact: None,
        }),
    )]);
    assert!(!has_conflict(&findings));
}

#[test]
fn invalid_dependency_requirement_conflicts() {
    let findings = cargo_findings(vec![(
        prov("p1"),
        dep_item_req(engine_core::ItemRequirements {
            required: vec![(
                cargo::DependencyRequirement {
                    file_key: None,
                    value: cargo::DependencySpec {
                        version: Some("1".to_owned()),
                        ..cargo::DependencySpec::default()
                    },
                },
                "invalid".to_owned(),
            )],
            forbidden: Vec::new(),
            allowed: None,
            exact: None,
        }),
    )]);
    assert!(has_conflict(&findings));
}

#[test]
fn patch_package_identity_requirement_without_file_key_is_unwritable() {
    let mut req = cargo::CargoTomlRequirements::default();
    let _ = req.patch.insert(
        "crates-io".to_owned(),
        engine_core::ItemRequirements {
            required: vec![(
                package_requirement("serde_json", Some("1")),
                "serde".to_owned(),
            )],
            forbidden: Vec::new(),
            allowed: None,
            exact: None,
        },
    );
    let findings = cargo_output(None, vec![(prov("p1"), req)]).findings;
    assert!(findings.iter().any(|finding| {
        matches!(finding, engine_core::Finding::UnwritableRequiredKey { key, .. } if key == "[patch.crates-io].serde_json")
    }));
}

#[test]
fn package_identity_dependency_is_satisfied_by_plain_key() {
    let req = dep_item_req(engine_core::ItemRequirements {
        required: vec![(
            package_requirement("serde_json", Some("1")),
            "serde".to_owned(),
        )],
        forbidden: Vec::new(),
        allowed: None,
        exact: None,
    });
    let findings = cargo_findings_with(
        Some(b"[dependencies]\nserde_json = \"1\"\n"),
        vec![(prov("p1"), req)],
    );
    assert!(findings.is_empty());
}

#[test]
fn package_identity_dependency_is_satisfied_by_rename() {
    dependency_package_identity_requirement_passes_under_renamed_key();
}

#[test]
fn local_key_dependency_is_not_satisfied_by_package_identity() {
    dependency_local_key_requirement_does_not_pass_on_package_only_match();
}

#[test]
fn missing_package_identity_dependency_writes_package_name() {
    let req = dep_item_req(engine_core::ItemRequirements {
        required: vec![(
            package_requirement("serde_json", Some("1")),
            "serde".to_owned(),
        )],
        forbidden: Vec::new(),
        allowed: None,
        exact: None,
    });
    let out = cargo_output(None, vec![(prov("p1"), req)]);
    let text =
        String::from_utf8(first_bytes(&out)).expect("engine output should be valid UTF-8 TOML");
    assert!(text.contains("[dependencies]"));
    assert!(text.contains("serde_json = \"1\""));
    assert!(!text.contains("package = \"serde_json\""));
}

#[test]
fn package_identity_dependency_init_reports_unwritable_when_package_key_is_reserved() {
    let req = dep_item_req(engine_core::ItemRequirements {
        required: vec![
            (
                package_requirement("serde_json", Some("1")),
                "serde".to_owned(),
            ),
            (
                local_dependency_requirement("serde_json", Some("serde_renamed"), Some("2")),
                "renamed".to_owned(),
            ),
        ],
        forbidden: Vec::new(),
        allowed: None,
        exact: None,
    });
    let out = cargo_output(None, vec![(prov("p1"), req)]);
    let text =
        String::from_utf8(first_bytes(&out)).expect("engine output should be valid UTF-8 TOML");
    assert!(out.findings.iter().any(|finding| {
        matches!(finding, engine_core::Finding::UnwritableRequiredKey { key, .. } if key == "[dependencies].serde_json")
    }));
    assert!(text.contains("package = \"serde_renamed\""));
    assert!(!text.contains("serde_json = \"1\""));
}

#[test]
fn package_identity_dependency_init_reports_unwritable_when_existing_key_is_different_package() {
    let req = dep_item_req(engine_core::ItemRequirements {
        required: vec![(
            package_requirement("serde_json", Some("1")),
            "serde".to_owned(),
        )],
        forbidden: Vec::new(),
        allowed: None,
        exact: None,
    });
    let out = cargo_output(
        Some(b"[dependencies]\nserde_json = { package = \"serde_renamed\", version = \"2\" }\n"),
        vec![(prov("p1"), req)],
    );
    let text =
        String::from_utf8(first_bytes(&out)).expect("engine output should be valid UTF-8 TOML");
    assert!(out.findings.iter().any(|finding| {
        matches!(finding, engine_core::Finding::UnwritableRequiredKey { key, .. } if key == "[dependencies].serde_json")
    }));
    assert!(text.contains("package = \"serde_renamed\""));
    assert!(!text.contains("serde_json = \"1\""));
}

#[test]
fn explicit_dependency_file_key_conflict_does_not_overwrite_package_identity() {
    let req = dep_item_req(engine_core::ItemRequirements {
        required: vec![
            (
                local_dependency_requirement("json", Some("serde_json"), Some("1")),
                "serde".to_owned(),
            ),
            (
                local_dependency_requirement("json", Some("serde_renamed"), Some("2")),
                "renamed".to_owned(),
            ),
        ],
        forbidden: Vec::new(),
        allowed: None,
        exact: None,
    });
    let out = cargo_output(None, vec![(prov("p1"), req)]);
    let text =
        String::from_utf8(first_bytes(&out)).expect("engine output should be valid UTF-8 TOML");
    assert!(out.findings.iter().any(|finding| {
        matches!(
            finding,
            engine_core::Finding::ConflictingRequirements { key, reason, .. }
                if key == "[dependencies].json"
                    && reason == "dependency-file-key-package-conflict"
        )
    }));
    assert_eq!(
        out.findings.len(),
        1,
        "merge conflicts must stop file reconciliation"
    );
    assert!(!text.contains("json ="));
}

#[test]
fn renamed_forbidden_package_reports_one_finding() {
    let req = dep_item_req(engine_core::ItemRequirements {
        required: Vec::new(),
        forbidden: vec![(
            package_requirement("serde_json", None),
            "no serde".to_owned(),
        )],
        allowed: None,
        exact: None,
    });
    let findings = cargo_findings_with(
        Some(b"[dependencies]\njson = { package = \"serde_json\", version = \"1\" }\n"),
        vec![(prov("p1"), req)],
    );
    assert_eq!(findings.len(), 1);
}

#[test]
fn forbidden_dependency_outside_allowed_reports_only_forbidden_classification() {
    let req = dep_item_req(engine_core::ItemRequirements {
        required: Vec::new(),
        forbidden: vec![(
            package_requirement("serde_json", None),
            "no serde".to_owned(),
        )],
        allowed: Some((
            vec![package_requirement("anyhow", None)],
            "only anyhow is allowed".to_owned(),
        )),
        exact: None,
    });
    let findings = cargo_findings_with(
        Some(b"[dependencies]\njson = { package = \"serde_json\", version = \"1\" }\n"),
        vec![(prov("p1"), req)],
    );

    assert_eq!(findings.len(), 1);
    assert!(matches!(
        findings.first(),
        Some(engine_core::Finding::Mismatch { expected, message, attribution, .. })
            if expected == "absent" && message == "no serde" && attribution == &[prov("p1")]
    ));
}

#[test]
fn closed_dependency_membership_removes_malformed_dependency_values() {
    for membership in [
        engine_core::ItemRequirements {
            allowed: Some((Vec::new(), "no dependencies are allowed".to_owned())),
            ..engine_core::ItemRequirements::default()
        },
        engine_core::ItemRequirements {
            exact: Some((Vec::new(), "dependencies must be empty".to_owned())),
            ..engine_core::ItemRequirements::default()
        },
    ] {
        let output = cargo_output(
            Some(b"[dependencies]\nevil = 1\n"),
            vec![(prov("p1"), dep_item_req(membership))],
        );

        assert_eq!(output.findings.len(), 1);
        assert!(
            !String::from_utf8(output.expected_bytes)
                .expect("engine output should remain UTF-8")
                .contains("evil")
        );
    }
}

#[test]
fn local_key_forbidden_dependency_removes_a_malformed_value() {
    let req = dep_item_req(engine_core::ItemRequirements {
        forbidden: vec![(
            local_dependency_requirement("evil", None, None),
            "evil is forbidden".to_owned(),
        )],
        ..engine_core::ItemRequirements::default()
    });
    let output = cargo_output(Some(b"[dependencies]\nevil = 1\n"), vec![(prov("p1"), req)]);

    assert_eq!(output.findings.len(), 1);
    assert!(
        !String::from_utf8(output.expected_bytes)
            .expect("engine output should remain UTF-8")
            .contains("evil")
    );
}

fn first_bytes(output: &aqc_file_engine_core::EngineOutput) -> Vec<u8> {
    output.expected_bytes.clone()
}
