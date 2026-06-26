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
    let (_, findings) = cargo::CargoTomlRequirements::merge(vec![(
        prov("p1"),
        dep_req(KeyedFixture {
            required,
            forbidden,
            closed: None,
        }),
    )]);
    assert!(findings.is_empty());
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
            closed: None,
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
        closed: None,
    });
    let (_, conflicts) = cargo::CargoTomlRequirements::merge(vec![(prov("p1"), req)]);
    assert!(conflicts.is_empty());
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
            closed: None,
        }),
    )]);
    assert!(has_conflict(&findings));
}

#[test]
fn closed_collection_rejects_outside_required_identity() {
    let closer = dep_item_req(engine_core::ItemRequirements {
        required: vec![(
            package_requirement("serde_json", Some("1")),
            "serde".to_owned(),
        )],
        forbidden: Vec::new(),
        closed: Some("only serde".to_owned()),
    });
    let outside = dep_item_req(engine_core::ItemRequirements {
        required: vec![(package_requirement("toml", Some("0.8")), "toml".to_owned())],
        forbidden: Vec::new(),
        closed: None,
    });
    let (_, conflicts) = cargo::CargoTomlRequirements::merge(vec![
        (prov("closer"), closer),
        (prov("outside"), outside),
    ]);
    assert!(conflicts.iter().any(|conflict| {
        conflict
            .contributors
            .iter()
            .any(|(p, value)| p.policy == "closer" && value == "closed")
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
        closed: None,
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
        closed: None,
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
        closed: None,
    });
    let out = cargo_output(
        Some(b"[dependencies]\njson = { package = \"serde_json\", version = \"1\" }\n"),
        vec![(prov("p1"), req)],
    );
    let text =
        String::from_utf8(out.expected_bytes).expect("engine output should be valid UTF-8 TOML");
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
            closed: None,
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
            closed: None,
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
        closed: None,
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
            closed: None,
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
            closed: None,
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
            closed: None,
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
        closed: None,
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
        closed: None,
    });
    let out = cargo_output(None, vec![(prov("p1"), req)]);
    let text =
        String::from_utf8(out.expected_bytes).expect("engine output should be valid UTF-8 TOML");
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
        closed: None,
    });
    let out = cargo_output(None, vec![(prov("p1"), req)]);
    let text =
        String::from_utf8(out.expected_bytes).expect("engine output should be valid UTF-8 TOML");
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
        closed: None,
    });
    let out = cargo_output(
        Some(b"[dependencies]\nserde_json = { package = \"serde_renamed\", version = \"2\" }\n"),
        vec![(prov("p1"), req)],
    );
    let text =
        String::from_utf8(out.expected_bytes).expect("engine output should be valid UTF-8 TOML");
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
        closed: None,
    });
    let out = cargo_output(None, vec![(prov("p1"), req)]);
    let text =
        String::from_utf8(out.expected_bytes).expect("engine output should be valid UTF-8 TOML");
    assert!(out.findings.iter().any(|finding| {
        matches!(
            finding,
            engine_core::Finding::ConflictingRequirements { key, reason, .. }
                if key == "[dependencies].json"
                    && reason == "dependency-file-key-package-conflict"
        )
    }));
    assert_eq!(
        out.findings
            .iter()
            .filter(|finding| {
                matches!(finding, engine_core::Finding::UnwritableRequiredKey { key, .. } if key == "[dependencies].json")
            })
            .count(),
        2
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
        closed: None,
    });
    let findings = cargo_findings_with(
        Some(b"[dependencies]\njson = { package = \"serde_json\", version = \"1\" }\n"),
        vec![(prov("p1"), req)],
    );
    assert_eq!(findings.len(), 1);
}
