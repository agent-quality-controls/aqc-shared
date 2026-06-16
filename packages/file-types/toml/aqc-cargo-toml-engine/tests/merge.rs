use std::collections::{BTreeMap, BTreeSet};

use toml_edit as _;

use aqc_cargo_toml_engine::{
    CargoTomlEngine, CargoTomlRequirements, DependencyIdentity, DependencyKind,
    DependencyRequirement, DependencyScope, DependencySpec, FeatureMembers, LintSetting,
    PackageFieldAssertion, PackageLintsAssertion, ProfileFieldAssertion, ProfileRequirements,
    ResolvedPackageFieldAssertion, TargetFieldAssertion, TargetRequirements, TargetTableAssertion,
};
use aqc_file_engine_core::{
    ConfigScalar, Engine, EngineRequirement, Finding, ItemRequirements, KeyedItem,
    ListRequirements, Provenance,
};

#[derive(Debug, Clone)]
struct KeyedFixture<Entry> {
    required: BTreeMap<String, (Entry, String)>,
    banned: BTreeMap<String, String>,
    closed: Option<String>,
}

fn prov(policy: &str) -> Provenance {
    Provenance {
        policy: policy.to_owned(),
    }
}

fn normal_scope() -> DependencyScope {
    DependencyScope {
        kind: DependencyKind::Normal,
        target: None,
    }
}

fn dep_spec(version: Option<&str>) -> DependencySpec {
    DependencySpec {
        version: version.map(str::to_owned),
        ..DependencySpec::default()
    }
}

fn dep_req(table: KeyedFixture<DependencySpec>) -> CargoTomlRequirements {
    let mut req = CargoTomlRequirements::default();
    let _ = req.dependencies.insert(normal_scope(), dep_items(table));
    req
}

fn dep_item_req(items: ItemRequirements<DependencyRequirement>) -> CargoTomlRequirements {
    let mut req = CargoTomlRequirements::default();
    let _ = req.dependencies.insert(normal_scope(), items);
    req
}

fn package_requirement(package: &str, version: Option<&str>) -> DependencyRequirement {
    DependencyRequirement {
        file_key: None,
        value: DependencySpec {
            package: Some(package.to_owned()),
            version: version.map(str::to_owned),
            ..DependencySpec::default()
        },
    }
}

fn local_dependency_requirement(
    file_key: &str,
    package: Option<&str>,
    version: Option<&str>,
) -> DependencyRequirement {
    DependencyRequirement {
        file_key: Some(file_key.to_owned()),
        value: DependencySpec {
            package: package.map(str::to_owned),
            version: version.map(str::to_owned),
            ..DependencySpec::default()
        },
    }
}

fn dep_items(table: KeyedFixture<DependencySpec>) -> ItemRequirements<DependencyRequirement> {
    ItemRequirements {
        required: table
            .required
            .into_iter()
            .map(|(file_key, (value, msg))| {
                (
                    DependencyRequirement {
                        file_key: Some(file_key),
                        value,
                    },
                    msg,
                )
            })
            .collect(),
        banned: table
            .banned
            .into_iter()
            .map(|(file_key, msg)| {
                (
                    DependencyRequirement {
                        file_key: Some(file_key),
                        value: DependencySpec::default(),
                    },
                    msg,
                )
            })
            .collect(),
        closed: table.closed,
    }
}

fn keyed_items<Entry: Default>(table: KeyedFixture<Entry>) -> ItemRequirements<KeyedItem<Entry>> {
    ItemRequirements {
        required: table
            .required
            .into_iter()
            .map(|(file_key, (value, msg))| (KeyedItem { file_key, value }, msg))
            .collect(),
        banned: table
            .banned
            .into_iter()
            .map(|(file_key, msg)| {
                (
                    KeyedItem {
                        file_key,
                        value: Entry::default(),
                    },
                    msg,
                )
            })
            .collect(),
        closed: table.closed,
    }
}

fn cargo_findings(reqs: Vec<(Provenance, CargoTomlRequirements)>) -> Vec<Finding> {
    cargo_findings_with(Some(b""), reqs)
}

fn cargo_findings_with(
    bytes: Option<&[u8]>,
    reqs: Vec<(Provenance, CargoTomlRequirements)>,
) -> Vec<Finding> {
    let reqs = reqs
        .into_iter()
        .map(|(p, r)| (p, Box::new(r) as Box<dyn EngineRequirement>))
        .collect::<Vec<_>>();
    CargoTomlEngine.reconcile(bytes, &reqs).findings
}

fn cargo_output(
    bytes: Option<&[u8]>,
    reqs: Vec<(Provenance, CargoTomlRequirements)>,
) -> aqc_file_engine_core::EngineOutput {
    let reqs = reqs
        .into_iter()
        .map(|(p, r)| (p, Box::new(r) as Box<dyn EngineRequirement>))
        .collect::<Vec<_>>();
    CargoTomlEngine.reconcile(bytes, &reqs)
}

fn has_conflict(findings: &[Finding]) -> bool {
    findings
        .iter()
        .any(|f| matches!(f, Finding::ConflictingRequirements { .. }))
}

#[test]
fn dependency_required_and_banned_different_keys_compose() {
    let mut required = BTreeMap::new();
    let _ = required.insert(
        "serde".to_owned(),
        (dep_spec(Some("1")), "serde".to_owned()),
    );
    let mut banned = BTreeMap::new();
    let _ = banned.insert("openssl".to_owned(), "no openssl".to_owned());
    let (_, findings) = CargoTomlRequirements::merge(vec![(
        prov("p1"),
        dep_req(KeyedFixture {
            required,
            banned,
            closed: None,
        }),
    )]);
    assert!(findings.is_empty());
}

#[test]
fn dependency_required_and_banned_same_key_conflicts() {
    let mut required = BTreeMap::new();
    let _ = required.insert(
        "serde".to_owned(),
        (dep_spec(Some("1")), "serde".to_owned()),
    );
    let mut banned = BTreeMap::new();
    let _ = banned.insert("serde".to_owned(), "no serde".to_owned());
    let findings = cargo_findings(vec![(
        prov("p1"),
        dep_req(KeyedFixture {
            required,
            banned,
            closed: None,
        }),
    )]);
    assert!(
        matches!(
            findings.first(),
            Some(Finding::ConflictingRequirements { .. })
        ) || has_conflict(&findings)
    );
}

#[test]
fn required_and_banned_different_dependency_identities_coexist() {
    let req = dep_item_req(ItemRequirements {
        required: vec![(
            package_requirement("serde_json", Some("1")),
            "serde".to_owned(),
        )],
        banned: vec![(
            package_requirement("openssl", None),
            "no openssl".to_owned(),
        )],
        closed: None,
    });
    let (_, conflicts) = CargoTomlRequirements::merge(vec![(prov("p1"), req)]);
    assert!(conflicts.is_empty());
}

#[test]
fn required_and_banned_same_dependency_identity_conflict() {
    let findings = cargo_findings(vec![(
        prov("p1"),
        dep_item_req(ItemRequirements {
            required: vec![(
                package_requirement("serde_json", Some("1")),
                "serde".to_owned(),
            )],
            banned: vec![(
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
    let closer = dep_item_req(ItemRequirements {
        required: vec![(
            package_requirement("serde_json", Some("1")),
            "serde".to_owned(),
        )],
        banned: Vec::new(),
        closed: Some("only serde".to_owned()),
    });
    let outside = dep_item_req(ItemRequirements {
        required: vec![(package_requirement("toml", Some("0.8")), "toml".to_owned())],
        banned: Vec::new(),
        closed: None,
    });
    let (_, conflicts) =
        CargoTomlRequirements::merge(vec![(prov("closer"), closer), (prov("outside"), outside)]);
    assert!(conflicts.iter().any(|conflict| {
        conflict
            .contributors
            .iter()
            .any(|(p, value)| p.policy == "closer" && value == "closed")
    }));
}

#[test]
fn dependency_local_key_requirement_does_not_pass_on_package_only_match() {
    let req = dep_item_req(ItemRequirements {
        required: vec![(
            local_dependency_requirement("json", Some("serde_json"), Some("1")),
            "need rename".to_owned(),
        )],
        banned: Vec::new(),
        closed: None,
    });
    let findings = cargo_findings_with(
        Some(b"[dependencies]\nserde_json = \"1\"\n"),
        vec![(prov("p1"), req)],
    );
    assert!(findings.iter().any(|finding| {
        matches!(finding, Finding::Mismatch { key, .. } if key == "[dependencies].json")
    }));
}

#[test]
fn dependency_package_identity_requirement_passes_under_renamed_key() {
    let req = dep_item_req(ItemRequirements {
        required: vec![(
            package_requirement("serde_json", Some("1")),
            "serde".to_owned(),
        )],
        banned: Vec::new(),
        closed: None,
    });
    let findings = cargo_findings_with(
        Some(b"[dependencies]\njson = { package = \"serde_json\", version = \"1\" }\n"),
        vec![(prov("p1"), req)],
    );
    assert!(findings.is_empty());
}

#[test]
fn dependency_package_identity_ban_catches_renamed_key() {
    let req = dep_item_req(ItemRequirements {
        required: Vec::new(),
        banned: vec![(
            package_requirement("serde_json", None),
            "no serde".to_owned(),
        )],
        closed: None,
    });
    let out = cargo_output(
        Some(b"[dependencies]\njson = { package = \"serde_json\", version = \"1\" }\n"),
        vec![(prov("p1"), req)],
    );
    let text = String::from_utf8(out.expected_bytes).expect("utf8");
    assert_eq!(out.findings.len(), 1);
    assert!(!text.contains("json ="));
}

#[test]
fn local_key_requirement_and_package_identity_ban_conflict() {
    let findings = cargo_findings(vec![(
        prov("p1"),
        dep_item_req(ItemRequirements {
            required: vec![(
                local_dependency_requirement("json", Some("serde_json"), Some("1")),
                "need rename".to_owned(),
            )],
            banned: vec![(
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
        dep_item_req(ItemRequirements {
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
            banned: Vec::new(),
            closed: None,
        }),
    )]);
    assert!(findings.iter().any(|finding| {
        matches!(
            finding,
            Finding::ConflictingRequirements { key, reason, .. }
                if key == "[dependencies].serde_json.file_key"
                    && reason == "dependency-package-multiple-file-keys"
        )
    }));
}

#[test]
fn package_identity_requirement_checks_all_duplicate_package_entries() {
    let req = dep_item_req(ItemRequirements {
        required: vec![(
            package_requirement("serde_json", Some("1")),
            "serde".to_owned(),
        )],
        banned: Vec::new(),
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
        dep_item_req(ItemRequirements {
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
            banned: Vec::new(),
            closed: None,
        }),
    )]);
    assert!(!has_conflict(&findings));
}

#[test]
fn invalid_dependency_requirement_conflicts() {
    let findings = cargo_findings(vec![(
        prov("p1"),
        dep_item_req(ItemRequirements {
            required: vec![(
                DependencyRequirement {
                    file_key: None,
                    value: DependencySpec {
                        version: Some("1".to_owned()),
                        ..DependencySpec::default()
                    },
                },
                "invalid".to_owned(),
            )],
            banned: Vec::new(),
            closed: None,
        }),
    )]);
    assert!(has_conflict(&findings));
}

#[test]
fn patch_package_identity_requirement_without_file_key_is_unwritable() {
    patch_package_identity_init_reports_unwritable_required_key();
}

#[test]
fn package_identity_dependency_is_satisfied_by_plain_key() {
    let req = dep_item_req(ItemRequirements {
        required: vec![(
            package_requirement("serde_json", Some("1")),
            "serde".to_owned(),
        )],
        banned: Vec::new(),
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
    let req = dep_item_req(ItemRequirements {
        required: vec![(
            package_requirement("serde_json", Some("1")),
            "serde".to_owned(),
        )],
        banned: Vec::new(),
        closed: None,
    });
    let out = cargo_output(None, vec![(prov("p1"), req)]);
    let text = String::from_utf8(out.expected_bytes).expect("utf8");
    assert!(text.contains("[dependencies]"));
    assert!(text.contains("serde_json = \"1\""));
    assert!(!text.contains("package = \"serde_json\""));
}

#[test]
fn package_identity_dependency_init_reports_unwritable_when_package_key_is_reserved() {
    let req = dep_item_req(ItemRequirements {
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
        banned: Vec::new(),
        closed: None,
    });
    let out = cargo_output(None, vec![(prov("p1"), req)]);
    let text = String::from_utf8(out.expected_bytes).expect("utf8");
    assert!(out.findings.iter().any(|finding| {
        matches!(finding, Finding::UnwritableRequiredKey { key, .. } if key == "[dependencies].serde_json")
    }));
    assert!(text.contains("package = \"serde_renamed\""));
    assert!(!text.contains("serde_json = \"1\""));
}

#[test]
fn package_identity_dependency_init_reports_unwritable_when_existing_key_is_different_package() {
    let req = dep_item_req(ItemRequirements {
        required: vec![(
            package_requirement("serde_json", Some("1")),
            "serde".to_owned(),
        )],
        banned: Vec::new(),
        closed: None,
    });
    let out = cargo_output(
        Some(b"[dependencies]\nserde_json = { package = \"serde_renamed\", version = \"2\" }\n"),
        vec![(prov("p1"), req)],
    );
    let text = String::from_utf8(out.expected_bytes).expect("utf8");
    assert!(out.findings.iter().any(|finding| {
        matches!(finding, Finding::UnwritableRequiredKey { key, .. } if key == "[dependencies].serde_json")
    }));
    assert!(text.contains("package = \"serde_renamed\""));
    assert!(!text.contains("serde_json = \"1\""));
}

#[test]
fn explicit_dependency_file_key_conflict_does_not_overwrite_package_identity() {
    let req = dep_item_req(ItemRequirements {
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
        banned: Vec::new(),
        closed: None,
    });
    let out = cargo_output(None, vec![(prov("p1"), req)]);
    let text = String::from_utf8(out.expected_bytes).expect("utf8");
    assert!(out.findings.iter().any(|finding| {
        matches!(
            finding,
            Finding::ConflictingRequirements { key, reason, .. }
                if key == "[dependencies].json"
                    && reason == "dependency-file-key-package-conflict"
        )
    }));
    assert_eq!(
        out.findings
            .iter()
            .filter(|finding| {
                matches!(finding, Finding::UnwritableRequiredKey { key, .. } if key == "[dependencies].json")
            })
            .count(),
        2
    );
    assert!(!text.contains("json ="));
}

#[test]
fn renamed_banned_package_reports_one_finding() {
    let req = dep_item_req(ItemRequirements {
        required: Vec::new(),
        banned: vec![(
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

#[test]
fn patch_package_identity_init_reports_unwritable_required_key() {
    let mut req = CargoTomlRequirements::default();
    let _ = req.patch.insert(
        "crates-io".to_owned(),
        ItemRequirements {
            required: vec![(
                package_requirement("serde_json", Some("1")),
                "serde".to_owned(),
            )],
            banned: Vec::new(),
            closed: None,
        },
    );
    let findings = cargo_output(None, vec![(prov("p1"), req)]).findings;
    assert!(findings.iter().any(|finding| {
        matches!(finding, Finding::UnwritableRequiredKey { key, .. } if key == "[patch.crates-io].serde_json")
    }));
}

#[test]
fn patch_requirement_with_file_key_writes_patch_entry() {
    let mut req = CargoTomlRequirements::default();
    let _ = req.patch.insert(
        "crates-io".to_owned(),
        ItemRequirements {
            required: vec![(
                DependencyRequirement {
                    file_key: Some("serde_json".to_owned()),
                    value: DependencySpec {
                        path: Some("../serde_json".to_owned()),
                        ..DependencySpec::default()
                    },
                },
                "patch serde".to_owned(),
            )],
            banned: Vec::new(),
            closed: None,
        },
    );
    let out = cargo_output(None, vec![(prov("p1"), req)]);
    let text = String::from_utf8(out.expected_bytes).expect("utf8");
    assert!(text.contains("[patch.crates-io]"));
    assert!(text.contains("serde_json = { path = \"../serde_json\" }"));
}

#[test]
fn dependency_partial_specs_compose_fieldwise() {
    let mut left = BTreeMap::new();
    let _ = left.insert(
        "serde".to_owned(),
        (
            DependencySpec {
                version: Some("1".to_owned()),
                ..DependencySpec::default()
            },
            "v".to_owned(),
        ),
    );
    let mut features = BTreeSet::new();
    let _ = features.insert("derive".to_owned());
    let mut right = BTreeMap::new();
    let _ = right.insert(
        "serde".to_owned(),
        (
            DependencySpec {
                features,
                ..DependencySpec::default()
            },
            "f".to_owned(),
        ),
    );
    let (merged, conflicts) = CargoTomlRequirements::merge(vec![
        (
            prov("p1"),
            dep_req(KeyedFixture {
                required: left,
                banned: BTreeMap::new(),
                closed: None,
            }),
        ),
        (
            prov("p2"),
            dep_req(KeyedFixture {
                required: right,
                banned: BTreeMap::new(),
                closed: None,
            }),
        ),
    ]);
    let spec: &DependencySpec = &merged.dependencies[&normal_scope()].required
        [&DependencyIdentity::LocalKey("serde".to_owned())]
        .merged
        .value;
    assert!(conflicts.is_empty());
    assert_eq!(spec.version.as_deref(), Some("1"));
    assert!(spec.features.contains("derive"));
}

#[test]
fn dependency_incompatible_same_field_specs_conflict() {
    let mut left = BTreeMap::new();
    let _ = left.insert("serde".to_owned(), (dep_spec(Some("1")), "one".to_owned()));
    let mut right = BTreeMap::new();
    let _ = right.insert("serde".to_owned(), (dep_spec(Some("2")), "two".to_owned()));
    let findings = cargo_findings(vec![
        (
            prov("p1"),
            dep_req(KeyedFixture {
                required: left,
                banned: BTreeMap::new(),
                closed: None,
            }),
        ),
        (
            prov("p2"),
            dep_req(KeyedFixture {
                required: right,
                banned: BTreeMap::new(),
                closed: None,
            }),
        ),
    ]);
    assert!(matches!(
        findings
            .iter()
            .find(|f| matches!(f, Finding::ConflictingRequirements { .. })),
        Some(Finding::ConflictingRequirements { .. })
    ));
}

#[test]
fn dependency_each_field_composes_independently() {
    let mut required = BTreeMap::new();
    let _ = required.insert(
        "serde".to_owned(),
        (
            DependencySpec {
                default_features: Some(false),
                optional: Some(true),
                registry: Some("crates-io".to_owned()),
                package: Some("serde-renamed".to_owned()),
                ..DependencySpec::default()
            },
            "attrs".to_owned(),
        ),
    );
    let (merged, conflicts) = CargoTomlRequirements::merge(vec![(
        prov("p1"),
        dep_req(KeyedFixture {
            required,
            banned: BTreeMap::new(),
            closed: None,
        }),
    )]);
    let spec: &DependencySpec = &merged.dependencies[&normal_scope()].required
        [&DependencyIdentity::Package("serde-renamed".to_owned())]
        .merged
        .value;
    assert!(conflicts.is_empty());
    assert_eq!(spec.default_features, Some(false));
    assert_eq!(spec.optional, Some(true));
    assert_eq!(spec.registry.as_deref(), Some("crates-io"));
    assert_eq!(spec.package.as_deref(), Some("serde-renamed"));
}

#[test]
fn cargo_lints_required_and_banned_different_keys_compose() {
    let mut required = BTreeMap::new();
    let entry = LintSetting {
        level: "deny".to_owned(),
        priority: None,
    };
    let _ = required.insert("unwrap_used".to_owned(), (entry, "unwrap".to_owned()));
    let mut banned = BTreeMap::new();
    let _ = banned.insert("dbg_macro".to_owned(), "no dbg".to_owned());
    let mut req = CargoTomlRequirements::default();
    let _ = req.workspace_lints.insert(
        "clippy".to_owned(),
        keyed_items(KeyedFixture {
            required,
            banned,
            closed: None,
        }),
    );
    let (_, findings) = CargoTomlRequirements::merge(vec![(prov("p1"), req)]);
    assert!(findings.is_empty());
}

#[test]
fn banned_cargo_lint_removes_malformed_existing_key() {
    let mut req = CargoTomlRequirements::default();
    let _ = req.workspace_lints.insert(
        "clippy".to_owned(),
        keyed_items(KeyedFixture::<LintSetting> {
            required: BTreeMap::new(),
            banned: BTreeMap::from([("unwrap_used".to_owned(), "no unwrap".to_owned())]),
            closed: None,
        }),
    );
    let out = cargo_output(
        Some(b"[workspace.lints.clippy]\nunwrap_used = 123\n"),
        vec![(prov("p1"), req)],
    );
    let text = String::from_utf8(out.expected_bytes).expect("utf8");
    assert!(!text.contains("unwrap_used"));
    assert_eq!(out.findings.len(), 1);
}

#[test]
fn cargo_features_use_table_composition_rules() {
    let mut names = BTreeSet::new();
    let _ = names.insert("dep:serde".to_owned());
    let entry = FeatureMembers { members: names };
    let mut required = BTreeMap::new();
    let _ = required.insert("default".to_owned(), (entry, "default".to_owned()));
    let table: KeyedFixture<FeatureMembers> = KeyedFixture {
        required,
        banned: BTreeMap::new(),
        closed: None,
    };
    let mut req = CargoTomlRequirements::default();
    req.features = Some(keyed_items(table));
    let (merged, conflicts) = CargoTomlRequirements::merge(vec![(prov("p1"), req)]);
    assert!(conflicts.is_empty());
    assert!(
        merged
            .features
            .expect("features")
            .required
            .contains_key("default")
    );
}

#[test]
fn table_closed_rejects_outside_required_entry_with_closer_attribution() {
    let closed = KeyedFixture::<DependencySpec> {
        required: BTreeMap::from([(
            "serde".to_owned(),
            (dep_spec(Some("1")), "serde".to_owned()),
        )]),
        banned: BTreeMap::new(),
        closed: Some("only serde".to_owned()),
    };
    let outside = KeyedFixture::<DependencySpec> {
        required: BTreeMap::from([("toml".to_owned(), (dep_spec(Some("1")), "toml".to_owned()))]),
        banned: BTreeMap::new(),
        closed: None,
    };
    let (_, conflicts) = CargoTomlRequirements::merge(vec![
        (prov("closer"), dep_req(closed)),
        (prov("outside"), dep_req(outside)),
    ]);
    let contributors = &conflicts[0].contributors;
    assert!(
        contributors
            .iter()
            .any(|(p, v)| p.policy == "closer" && v == "closed")
    );
}

#[test]
fn table_closed_allows_outside_banned_entry() {
    let table = KeyedFixture::<DependencySpec> {
        required: BTreeMap::new(),
        banned: BTreeMap::from([("openssl".to_owned(), "ban".to_owned())]),
        closed: Some("closed".to_owned()),
    };
    let (_, conflicts) = CargoTomlRequirements::merge(vec![(prov("closer"), dep_req(table))]);
    assert!(conflicts.is_empty());
}

#[test]
fn table_two_closed_tables_with_different_required_keys_conflict() {
    let closed = Some("closed".to_owned());
    let left = KeyedFixture {
        required: BTreeMap::from([(
            "serde".to_owned(),
            (dep_spec(Some("1")), "serde".to_owned()),
        )]),
        banned: BTreeMap::new(),
        closed: closed.clone(),
    };
    let right = KeyedFixture {
        required: BTreeMap::from([("toml".to_owned(), (dep_spec(Some("1")), "toml".to_owned()))]),
        banned: BTreeMap::new(),
        closed,
    };
    let findings = cargo_findings(vec![
        (prov("p1"), dep_req(left)),
        (prov("p2"), dep_req(right)),
    ]);
    assert!(
        findings
            .iter()
            .any(|f| matches!(f, Finding::ConflictingRequirements { .. }))
    );
}

#[test]
fn table_same_required_key_compatible_entries_compose() {
    let mut left = BTreeMap::new();
    let _ = left.insert(
        "serde".to_owned(),
        (dep_spec(Some("1")), "serde".to_owned()),
    );
    let mut features = BTreeSet::new();
    let _ = features.insert("derive".to_owned());
    let mut right = BTreeMap::new();
    let _ = right.insert(
        "serde".to_owned(),
        (
            DependencySpec {
                features,
                ..DependencySpec::default()
            },
            "features".to_owned(),
        ),
    );
    let (_, findings) = CargoTomlRequirements::merge(vec![
        (
            prov("p1"),
            dep_req(KeyedFixture {
                required: left,
                banned: BTreeMap::new(),
                closed: None,
            }),
        ),
        (
            prov("p2"),
            dep_req(KeyedFixture {
                required: right,
                banned: BTreeMap::new(),
                closed: None,
            }),
        ),
    ]);
    assert!(findings.is_empty());
}

#[test]
fn table_same_required_key_incompatible_entries_conflict() {
    let findings = cargo_findings(vec![
        (
            prov("p1"),
            dep_req(KeyedFixture {
                required: BTreeMap::from([(
                    "serde".to_owned(),
                    (dep_spec(Some("1")), "one".to_owned()),
                )]),
                banned: BTreeMap::new(),
                closed: None,
            }),
        ),
        (
            prov("p2"),
            dep_req(KeyedFixture {
                required: BTreeMap::from([(
                    "serde".to_owned(),
                    (dep_spec(Some("2")), "two".to_owned()),
                )]),
                banned: BTreeMap::new(),
                closed: None,
            }),
        ),
    ]);
    assert!(
        findings
            .iter()
            .any(|f| matches!(f, Finding::ConflictingRequirements { .. }))
    );
}

#[test]
fn list_contains_and_excludes_different_items_compose() {
    let list = ListRequirements {
        contains: BTreeMap::from([("derive".to_owned(), "need derive".to_owned())]),
        excludes: BTreeMap::from([("rc".to_owned(), "no rc".to_owned())]),
        exact: None,
    };
    let mut req = CargoTomlRequirements::default();
    let _ = req
        .package_fields
        .insert("keywords".to_owned(), PackageFieldAssertion::List(list));
    let (_, conflicts) = CargoTomlRequirements::merge(vec![(prov("p1"), req)]);
    assert!(conflicts.is_empty());
}

#[test]
fn list_contains_and_excludes_same_item_conflicts() {
    let list = ListRequirements {
        contains: BTreeMap::from([("derive".to_owned(), "need".to_owned())]),
        excludes: BTreeMap::from([("derive".to_owned(), "ban".to_owned())]),
        exact: None,
    };
    let mut req = CargoTomlRequirements::default();
    let _ = req
        .package_fields
        .insert("keywords".to_owned(), PackageFieldAssertion::List(list));
    let findings = cargo_findings(vec![(prov("p1"), req)]);
    assert!(matches!(
        findings
            .iter()
            .find(|f| matches!(f, Finding::ConflictingRequirements { .. })),
        Some(Finding::ConflictingRequirements { .. })
    ));
}

#[test]
fn list_contains_and_exact_compose_when_exact_contains_item() {
    let exact = Some((
        vec!["derive".to_owned(), "std".to_owned()],
        "exact".to_owned(),
    ));
    let list = ListRequirements {
        contains: BTreeMap::from([("derive".to_owned(), "need".to_owned())]),
        excludes: BTreeMap::new(),
        exact,
    };
    let mut req = CargoTomlRequirements::default();
    let _ = req
        .package_fields
        .insert("keywords".to_owned(), PackageFieldAssertion::List(list));
    let (_, conflicts) = CargoTomlRequirements::merge(vec![(prov("p1"), req)]);
    assert!(conflicts.is_empty());
}

#[test]
fn list_excludes_and_exact_compose_when_exact_omits_item() {
    let list = ListRequirements {
        contains: BTreeMap::new(),
        excludes: BTreeMap::from([("rc".to_owned(), "no rc".to_owned())]),
        exact: Some((vec!["derive".to_owned()], "exact".to_owned())),
    };
    let mut req = CargoTomlRequirements::default();
    let _ = req
        .package_fields
        .insert("keywords".to_owned(), PackageFieldAssertion::List(list));
    let (_, conflicts) = CargoTomlRequirements::merge(vec![(prov("p1"), req)]);
    assert!(conflicts.is_empty());
}

#[test]
fn profiles_attribute_only_failed_field_assertions() {
    let mut profile = ProfileRequirements::default();
    let _ = profile.fields.insert(
        "opt-level".to_owned(),
        ProfileFieldAssertion::Equals(ConfigScalar::Int(3), "opt".to_owned()),
    );
    let mut req = CargoTomlRequirements::default();
    let _ = req.profiles.insert("release".to_owned(), profile);
    let findings = cargo_findings(vec![(prov("profile-policy"), req)]);
    let contributors = findings
        .iter()
        .filter_map(|f| match f {
            Finding::Mismatch { attribution, .. } => Some(attribution),
            _ => None,
        })
        .flatten()
        .collect::<Vec<_>>();
    assert!(contributors.iter().all(|p| p.policy == "profile-policy"));
}

#[test]
fn target_tables_attribute_only_failed_field_assertions() {
    let mut path_targets = TargetRequirements::default();
    let _ = path_targets.bin_targets.insert(
        "cli".to_owned(),
        TargetTableAssertion::Fields(BTreeMap::from([(
            "path".to_owned(),
            TargetFieldAssertion::Equals(
                ConfigScalar::Str("src/bin/cli.rs".to_owned()),
                "path".to_owned(),
            ),
        )])),
    );
    let mut harness_targets = TargetRequirements::default();
    let _ = harness_targets.bin_targets.insert(
        "cli".to_owned(),
        TargetTableAssertion::Fields(BTreeMap::from([(
            "harness".to_owned(),
            TargetFieldAssertion::Equals(ConfigScalar::Bool(true), "harness".to_owned()),
        )])),
    );
    let mut path_req = CargoTomlRequirements::default();
    path_req.targets = path_targets;
    let mut harness_req = CargoTomlRequirements::default();
    harness_req.targets = harness_targets;
    let findings = cargo_findings_with(
        Some(
            br#"[[bin]]
name = "cli"
harness = true
"#,
        ),
        vec![
            (prov("path-policy"), path_req),
            (prov("harness-policy"), harness_req),
        ],
    );
    let contributors = findings
        .iter()
        .filter_map(|f| match f {
            Finding::Mismatch { attribution, .. } => Some(attribution),
            _ => None,
        })
        .flatten()
        .collect::<Vec<_>>();
    assert!(contributors.iter().any(|p| p.policy == "path-policy"));
    assert!(contributors.iter().all(|p| p.policy != "harness-policy"));
}

#[test]
fn list_findings_use_per_item_attribution() {
    let list = ListRequirements {
        contains: BTreeMap::from([("derive".to_owned(), "need derive".to_owned())]),
        excludes: BTreeMap::new(),
        exact: None,
    };
    let mut contains_req = CargoTomlRequirements::default();
    let _ = contains_req
        .package_fields
        .insert("keywords".to_owned(), PackageFieldAssertion::List(list));

    let list = ListRequirements {
        contains: BTreeMap::new(),
        excludes: BTreeMap::from([("rc".to_owned(), "no rc".to_owned())]),
        exact: None,
    };
    let mut excludes_req = CargoTomlRequirements::default();
    let _ = excludes_req
        .package_fields
        .insert("keywords".to_owned(), PackageFieldAssertion::List(list));

    let findings = cargo_findings_with(
        Some(
            br#"[package]
keywords = ["rc"]
"#,
        ),
        vec![
            (prov("contains-policy"), contains_req),
            (prov("excludes-policy"), excludes_req),
        ],
    );
    let contains = findings.iter().find(
        |finding| matches!(finding, Finding::Mismatch { message, .. } if message == "need derive"),
    );
    let excludes = findings
        .iter()
        .find(|finding| matches!(finding, Finding::Mismatch { message, .. } if message == "no rc"));
    assert!(matches!(
        contains,
        Some(Finding::Mismatch { attribution, .. }) if attribution.iter().all(|p| p.policy == "contains-policy")
    ));
    assert!(matches!(
        excludes,
        Some(Finding::Mismatch { attribution, .. }) if attribution.iter().all(|p| p.policy == "excludes-policy")
    ));
}

#[test]
fn target_table_list_fields_use_unified_list_rules() {
    let list = ListRequirements {
        contains: BTreeMap::from([("feat-a".to_owned(), "need".to_owned())]),
        excludes: BTreeMap::new(),
        exact: None,
    };
    let field = TargetFieldAssertion::List(list);
    let mut targets = TargetRequirements::default();
    let _ = targets
        .lib_fields
        .insert("required-features".to_owned(), field);
    let mut req = CargoTomlRequirements::default();
    req.targets = targets;
    let (_, conflicts) = CargoTomlRequirements::merge(vec![(prov("p1"), req)]);
    assert!(conflicts.is_empty());
}

#[test]
fn scalar_implication_cases_compose() {
    let mut set = BTreeSet::new();
    let _ = set.insert("2021".to_owned());
    let exact =
        PackageFieldAssertion::Equals(ConfigScalar::Str("2021".to_owned()), "exact".to_owned());
    let one = PackageFieldAssertion::OneOf(set, "one".to_owned());
    let present = PackageFieldAssertion::Present("present".to_owned());
    let mut left = CargoTomlRequirements::default();
    let _ = left.package_fields.insert("edition".to_owned(), exact);
    let mut right = CargoTomlRequirements::default();
    let _ = right.package_fields.insert("edition".to_owned(), one);
    let mut third = CargoTomlRequirements::default();
    let _ = third.package_fields.insert("edition".to_owned(), present);
    let (merged, conflicts) = CargoTomlRequirements::merge(vec![
        (prov("p1"), left),
        (prov("p2"), right),
        (prov("p3"), third),
    ]);
    assert!(conflicts.is_empty());
    assert!(matches!(
        merged.package_fields["edition"].merged,
        ResolvedPackageFieldAssertion::Equals(ConfigScalar::Str(ref value), _) if value == "2021"
    ));
}

#[test]
fn scalar_implication_attributes_only_failed_assertions() {
    let mut allowed = BTreeSet::new();
    let _ = allowed.insert("2021".to_owned());
    let _ = allowed.insert("2024".to_owned());
    let mut exact_policy = CargoTomlRequirements::default();
    let _ = exact_policy.package_fields.insert(
        "edition".to_owned(),
        PackageFieldAssertion::Equals(ConfigScalar::Str("2021".to_owned()), "exact".to_owned()),
    );
    let mut oneof_policy = CargoTomlRequirements::default();
    let _ = oneof_policy.package_fields.insert(
        "edition".to_owned(),
        PackageFieldAssertion::OneOf(allowed, "one".to_owned()),
    );
    let mut present_policy = CargoTomlRequirements::default();
    let _ = present_policy.package_fields.insert(
        "edition".to_owned(),
        PackageFieldAssertion::Present("present".to_owned()),
    );
    let findings = cargo_findings_with(
        Some(
            br#"[package]
edition = "2024"
"#,
        ),
        vec![
            (prov("exact-policy"), exact_policy),
            (prov("oneof-policy"), oneof_policy),
            (prov("present-policy"), present_policy),
        ],
    );
    let mismatch = findings
        .iter()
        .find(|finding| matches!(finding, Finding::Mismatch { message, .. } if message == "exact"))
        .expect("exact mismatch");
    assert!(matches!(
        mismatch,
        Finding::Mismatch { attribution, .. }
            if attribution.iter().any(|p| p.policy == "exact-policy")
                && attribution.iter().all(|p| p.policy != "oneof-policy" && p.policy != "present-policy")
    ));
}

#[test]
fn workspace_inheritance_composes_with_present() {
    let mut left = CargoTomlRequirements::default();
    let _ = left.package_fields.insert(
        "version".to_owned(),
        PackageFieldAssertion::Present("present".to_owned()),
    );
    let mut right = CargoTomlRequirements::default();
    let _ = right.package_fields.insert(
        "version".to_owned(),
        PackageFieldAssertion::InheritsWorkspace("inherit".to_owned()),
    );
    let (merged, conflicts) =
        CargoTomlRequirements::merge(vec![(prov("p1"), left), (prov("p2"), right)]);
    assert!(conflicts.is_empty());
    assert!(matches!(
        merged.package_fields["version"].merged,
        ResolvedPackageFieldAssertion::InheritsWorkspace(ref msg) if msg == "inherit"
    ));
}

#[test]
fn cargo_scalar_incompatible_cases_conflict() {
    let mut set = BTreeSet::new();
    let _ = set.insert("2018".to_owned());
    let mut left = CargoTomlRequirements::default();
    let _ = left.package_fields.insert(
        "edition".to_owned(),
        PackageFieldAssertion::Equals(ConfigScalar::Str("2021".to_owned()), "2021".to_owned()),
    );
    let mut right = CargoTomlRequirements::default();
    let _ = right.package_fields.insert(
        "edition".to_owned(),
        PackageFieldAssertion::OneOf(set, "2018 only".to_owned()),
    );
    let findings = cargo_findings(vec![(prov("p1"), left), (prov("p2"), right)]);
    assert!(
        findings
            .iter()
            .any(|f| matches!(f, Finding::ConflictingRequirements { .. }))
    );
}

#[test]
fn cargo_at_least_version_keeps_strongest_floor() {
    let mut left = CargoTomlRequirements::default();
    let _ = left.package_fields.insert(
        "rust-version".to_owned(),
        PackageFieldAssertion::AtLeastVersion("1.80".to_owned(), "old".to_owned()),
    );
    let mut right = CargoTomlRequirements::default();
    let _ = right.package_fields.insert(
        "rust-version".to_owned(),
        PackageFieldAssertion::AtLeastVersion("1.85".to_owned(), "new".to_owned()),
    );
    let (merged, conflicts) =
        CargoTomlRequirements::merge(vec![(prov("p1"), left), (prov("p2"), right)]);
    let ResolvedPackageFieldAssertion::AtLeastVersion(version, _) =
        &merged.package_fields["rust-version"].merged
    else {
        panic!("expected AtLeastVersion");
    };
    assert!(conflicts.is_empty());
    assert_eq!(version, "1.85");
}

#[test]
fn package_lints_inline_preserves_per_lint_attribution() {
    let table = KeyedFixture {
        required: BTreeMap::from([(
            "unwrap_used".to_owned(),
            (
                LintSetting {
                    level: "deny".to_owned(),
                    priority: None,
                },
                "outer".to_owned(),
            ),
        )]),
        banned: BTreeMap::new(),
        closed: None,
    };
    let mut req = CargoTomlRequirements::default();
    req.package_lints = Some(PackageLintsAssertion::Inline(BTreeMap::from([(
        "clippy".to_owned(),
        keyed_items(table),
    )])));
    let findings = cargo_findings(vec![(prov("lint-policy"), req)]);
    let contributors = findings
        .iter()
        .filter_map(|f| match f {
            Finding::Mismatch { attribution, .. } => Some(attribution),
            _ => None,
        })
        .flatten()
        .collect::<Vec<_>>();
    assert!(contributors.iter().all(|p| p.policy == "lint-policy"));
}
