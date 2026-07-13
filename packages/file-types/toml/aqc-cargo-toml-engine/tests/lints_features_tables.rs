#![allow(
    clippy::expect_used,
    reason = "Tests use expect to fail loudly when fixture invariants are broken."
)]
#![allow(
    clippy::field_reassign_with_default,
    clippy::indexing_slicing,
    reason = "These tests keep compact Cargo requirement fixtures and index known conflict entries."
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
fn cargo_lints_required_and_forbidden_different_keys_compose() {
    let mut required = BTreeMap::new();
    let entry = cargo::LintSetting {
        level: "deny".to_owned(),
        priority: None,
    };
    let _ = required.insert("unwrap_used".to_owned(), (entry, "unwrap".to_owned()));
    let mut forbidden = BTreeMap::new();
    let _ = forbidden.insert("dbg_macro".to_owned(), "no dbg".to_owned());
    let mut req = cargo::CargoTomlRequirements::default();
    let _ = req.workspace_lints.insert(
        "clippy".to_owned(),
        keyed_items(KeyedFixture {
            required,
            forbidden,
            exact: None,
        }),
    );
    let _resolved = cargo::CargoTomlRequirements::merge(vec![(prov("p1"), req)])
        .expect("different lint identities should merge");
}

#[test]
fn forbidden_cargo_lint_removes_malformed_existing_key() {
    let mut req = cargo::CargoTomlRequirements::default();
    let _ = req.workspace_lints.insert(
        "clippy".to_owned(),
        keyed_items(KeyedFixture::<cargo::LintSetting> {
            required: BTreeMap::new(),
            forbidden: BTreeMap::from([("unwrap_used".to_owned(), "no unwrap".to_owned())]),
            exact: None,
        }),
    );
    let out = cargo_output(
        Some(b"[workspace.lints.clippy]\nunwrap_used = 123\n"),
        vec![(prov("p1"), req)],
    );
    let text =
        String::from_utf8(first_bytes(&out)).expect("engine output should be valid UTF-8 TOML");
    assert!(!text.contains("unwrap_used"));
    assert_eq!(out.findings.len(), 1);
}

#[test]
fn cargo_features_use_table_composition_rules() {
    let mut names = BTreeSet::new();
    let _ = names.insert("dep:serde".to_owned());
    let entry = cargo::FeatureMembers { members: names };
    let mut required = BTreeMap::new();
    let _ = required.insert("default".to_owned(), (entry, "default".to_owned()));
    let table: KeyedFixture<cargo::FeatureMembers> = KeyedFixture {
        required,
        forbidden: BTreeMap::new(),
        exact: None,
    };
    let mut req = cargo::CargoTomlRequirements::default();
    req.features = Some(keyed_items(table));
    let merged = cargo::CargoTomlRequirements::merge(vec![(prov("p1"), req)])
        .expect("feature requirement should merge");
    assert!(
        merged
            .features()
            .as_ref()
            .expect("features")
            .required
            .contains_key("default")
    );
}

#[test]
fn table_exact_rejects_outside_required_entry_with_exact_attribution() {
    let exact = KeyedFixture::<cargo::DependencySpec> {
        required: BTreeMap::from([(
            "serde".to_owned(),
            (dep_spec(Some("1")), "serde".to_owned()),
        )]),
        forbidden: BTreeMap::new(),
        exact: Some("only serde".to_owned()),
    };
    let outside = KeyedFixture::<cargo::DependencySpec> {
        required: BTreeMap::from([("toml".to_owned(), (dep_spec(Some("1")), "toml".to_owned()))]),
        forbidden: BTreeMap::new(),
        exact: None,
    };
    let conflicts = cargo::CargoTomlRequirements::merge(vec![
        (prov("closer"), dep_req(exact)),
        (prov("outside"), dep_req(outside)),
    ])
    .expect_err("required item outside exact set should conflict");
    let contributors = &conflicts[0].contributors;
    assert!(
        contributors
            .iter()
            .any(|(p, v)| p.policy == "closer" && v == "only serde")
    );
}

#[test]
fn table_exact_allows_outside_forbidden_entry() {
    let table = KeyedFixture::<cargo::DependencySpec> {
        required: BTreeMap::new(),
        forbidden: BTreeMap::from([("openssl".to_owned(), "forbid".to_owned())]),
        exact: Some("exact".to_owned()),
    };
    let _resolved = cargo::CargoTomlRequirements::merge(vec![(prov("closer"), dep_req(table))])
        .expect("forbidden item outside exact set should merge");
}

#[test]
fn table_two_exact_tables_with_different_required_keys_conflict() {
    let exact = Some("exact".to_owned());
    let left = KeyedFixture {
        required: BTreeMap::from([(
            "serde".to_owned(),
            (dep_spec(Some("1")), "serde".to_owned()),
        )]),
        forbidden: BTreeMap::new(),
        exact: exact.clone(),
    };
    let right = KeyedFixture {
        required: BTreeMap::from([("toml".to_owned(), (dep_spec(Some("1")), "toml".to_owned()))]),
        forbidden: BTreeMap::new(),
        exact,
    };
    let findings = cargo_findings(vec![
        (prov("p1"), dep_req(left)),
        (prov("p2"), dep_req(right)),
    ]);
    assert!(
        findings
            .iter()
            .any(|f| matches!(f, engine_core::Finding::ConflictingRequirements { .. }))
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
            cargo::DependencySpec {
                features,
                ..cargo::DependencySpec::default()
            },
            "features".to_owned(),
        ),
    );
    let _resolved = cargo::CargoTomlRequirements::merge(vec![
        (
            prov("p1"),
            dep_req(KeyedFixture {
                required: left,
                forbidden: BTreeMap::new(),
                exact: None,
            }),
        ),
        (
            prov("p2"),
            dep_req(KeyedFixture {
                required: right,
                forbidden: BTreeMap::new(),
                exact: None,
            }),
        ),
    ])
    .expect("compatible dependency features should merge");
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
                forbidden: BTreeMap::new(),
                exact: None,
            }),
        ),
        (
            prov("p2"),
            dep_req(KeyedFixture {
                required: BTreeMap::from([(
                    "serde".to_owned(),
                    (dep_spec(Some("2")), "two".to_owned()),
                )]),
                forbidden: BTreeMap::new(),
                exact: None,
            }),
        ),
    ]);
    assert!(
        findings
            .iter()
            .any(|f| matches!(f, engine_core::Finding::ConflictingRequirements { .. }))
    );
}

#[test]
fn list_contains_and_excludes_different_items_compose() {
    let list = engine_core::ListRequirements {
        contains: BTreeMap::from([("derive".to_owned(), "need derive".to_owned())]),
        excludes: BTreeMap::from([("rc".to_owned(), "no rc".to_owned())]),
        exact: None,
    };
    let mut req = cargo::CargoTomlRequirements::default();
    let _ = req.package_fields.insert(
        "keywords".to_owned(),
        cargo::PackageFieldAssertion::List(list),
    );
    let _resolved = cargo::CargoTomlRequirements::merge(vec![(prov("p1"), req)])
        .expect("compatible list requirements should merge");
}

#[test]
fn list_contains_and_excludes_same_item_conflicts() {
    let list = engine_core::ListRequirements {
        contains: BTreeMap::from([("derive".to_owned(), "need".to_owned())]),
        excludes: BTreeMap::from([("derive".to_owned(), "forbid".to_owned())]),
        exact: None,
    };
    let mut req = cargo::CargoTomlRequirements::default();
    let _ = req.package_fields.insert(
        "keywords".to_owned(),
        cargo::PackageFieldAssertion::List(list),
    );
    let findings = cargo_findings(vec![(prov("p1"), req)]);
    assert!(matches!(
        findings
            .iter()
            .find(|f| matches!(f, engine_core::Finding::ConflictingRequirements { .. })),
        Some(engine_core::Finding::ConflictingRequirements { .. })
    ));
}

#[test]
fn list_contains_and_exact_compose_when_exact_contains_item() {
    let exact = Some((
        vec!["derive".to_owned(), "std".to_owned()],
        "exact".to_owned(),
    ));
    let list = engine_core::ListRequirements {
        contains: BTreeMap::from([("derive".to_owned(), "need".to_owned())]),
        excludes: BTreeMap::new(),
        exact,
    };
    let mut req = cargo::CargoTomlRequirements::default();
    let _ = req.package_fields.insert(
        "keywords".to_owned(),
        cargo::PackageFieldAssertion::List(list),
    );
    let _resolved = cargo::CargoTomlRequirements::merge(vec![(prov("p1"), req)])
        .expect("compatible exact list should merge");
}

#[test]
fn list_excludes_and_exact_compose_when_exact_omits_item() {
    let list = engine_core::ListRequirements {
        contains: BTreeMap::new(),
        excludes: BTreeMap::from([("rc".to_owned(), "no rc".to_owned())]),
        exact: Some((vec!["derive".to_owned()], "exact".to_owned())),
    };
    let mut req = cargo::CargoTomlRequirements::default();
    let _ = req.package_fields.insert(
        "keywords".to_owned(),
        cargo::PackageFieldAssertion::List(list),
    );
    let _resolved = cargo::CargoTomlRequirements::merge(vec![(prov("p1"), req)])
        .expect("excluded item omitted by exact list should merge");
}

fn first_bytes(output: &aqc_file_engine_core::EngineOutput) -> Vec<u8> {
    output.expected_bytes.clone()
}
