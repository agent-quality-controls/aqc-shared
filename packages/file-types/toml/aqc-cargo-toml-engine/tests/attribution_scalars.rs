#![expect(
    clippy::field_reassign_with_default,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::shadow_unrelated,
    clippy::wildcard_enum_match_arm,
    reason = "Attribution tests keep compact fixtures and pattern assertions near expected findings."
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
fn profiles_attribute_only_failed_field_assertions() {
    let mut profile = cargo::ProfileRequirements::default();
    let _ = profile.fields.insert(
        "opt-level".to_owned(),
        engine_core::ScalarAssertion::Equals(engine_core::ConfigScalar::Int(3), "opt".to_owned()),
    );
    let mut req = cargo::CargoTomlRequirements::default();
    let _ = req.profiles.insert("release".to_owned(), profile);
    let findings = cargo_findings(vec![(prov("profile-policy"), req)]);
    let contributors = findings
        .iter()
        .filter_map(|f| match f {
            engine_core::Finding::Mismatch { attribution, .. } => Some(attribution),
            _ => None,
        })
        .flatten()
        .collect::<Vec<_>>();
    assert!(contributors.iter().all(|p| p.policy == "profile-policy"));
}

#[test]
fn target_tables_attribute_only_failed_field_assertions() {
    let mut path_targets = cargo::TargetRequirements::default();
    let _ = path_targets.bin_targets.insert(
        "cli".to_owned(),
        cargo::TargetTableAssertion::Fields(BTreeMap::from([(
            "path".to_owned(),
            cargo::TargetFieldAssertion::Scalar(engine_core::ScalarAssertion::Equals(
                engine_core::ConfigScalar::Str("src/bin/cli.rs".to_owned()),
                "path".to_owned(),
            )),
        )])),
    );
    let mut harness_targets = cargo::TargetRequirements::default();
    let _ = harness_targets.bin_targets.insert(
        "cli".to_owned(),
        cargo::TargetTableAssertion::Fields(BTreeMap::from([(
            "harness".to_owned(),
            cargo::TargetFieldAssertion::Scalar(engine_core::ScalarAssertion::Equals(
                engine_core::ConfigScalar::Bool(true),
                "harness".to_owned(),
            )),
        )])),
    );
    let mut path_req = cargo::CargoTomlRequirements::default();
    path_req.targets = path_targets;
    let mut harness_req = cargo::CargoTomlRequirements::default();
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
            engine_core::Finding::Mismatch { attribution, .. } => Some(attribution),
            _ => None,
        })
        .flatten()
        .collect::<Vec<_>>();
    assert!(contributors.iter().any(|p| p.policy == "path-policy"));
    assert!(contributors.iter().all(|p| p.policy != "harness-policy"));
}

#[test]
fn list_findings_use_per_item_attribution() {
    let list = engine_core::ListRequirements {
        contains: BTreeMap::from([("derive".to_owned(), "need derive".to_owned())]),
        excludes: BTreeMap::new(),
        exact: None,
    };
    let mut contains_req = cargo::CargoTomlRequirements::default();
    let _ = contains_req.package_fields.insert(
        "keywords".to_owned(),
        cargo::PackageFieldAssertion::List(list),
    );

    let list = engine_core::ListRequirements {
        contains: BTreeMap::new(),
        excludes: BTreeMap::from([("rc".to_owned(), "no rc".to_owned())]),
        exact: None,
    };
    let mut excludes_req = cargo::CargoTomlRequirements::default();
    let _ = excludes_req.package_fields.insert(
        "keywords".to_owned(),
        cargo::PackageFieldAssertion::List(list),
    );

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
        |finding| matches!(finding, engine_core::Finding::Mismatch { message, .. } if message == "need derive"),
    );
    let excludes = findings
        .iter()
        .find(|finding| matches!(finding, engine_core::Finding::Mismatch { message, .. } if message == "no rc"));
    assert!(matches!(
        contains,
        Some(engine_core::Finding::Mismatch { attribution, .. }) if attribution.iter().all(|p| p.policy == "contains-policy")
    ));
    assert!(matches!(
        excludes,
        Some(engine_core::Finding::Mismatch { attribution, .. }) if attribution.iter().all(|p| p.policy == "excludes-policy")
    ));
}

#[test]
fn target_table_list_fields_use_unified_list_rules() {
    let list = engine_core::ListRequirements {
        contains: BTreeMap::from([("feat-a".to_owned(), "need".to_owned())]),
        excludes: BTreeMap::new(),
        exact: None,
    };
    let field = cargo::TargetFieldAssertion::List(list);
    let mut targets = cargo::TargetRequirements::default();
    let _ = targets
        .lib_fields
        .insert("required-features".to_owned(), field);
    let mut req = cargo::CargoTomlRequirements::default();
    req.targets = targets;
    let (_, conflicts) = cargo::CargoTomlRequirements::merge(vec![(prov("p1"), req)]);
    assert!(conflicts.is_empty());
}

#[test]
fn scalar_implication_cases_compose() {
    let mut set = BTreeSet::new();
    let _ = set.insert(engine_core::ConfigScalar::Str("2021".to_owned()));
    let exact = cargo::PackageFieldAssertion::Scalar(engine_core::ScalarAssertion::Equals(
        engine_core::ConfigScalar::Str("2021".to_owned()),
        "exact".to_owned(),
    ));
    let one = cargo::PackageFieldAssertion::Scalar(engine_core::ScalarAssertion::OneOf(
        set,
        "one".to_owned(),
    ));
    let present = cargo::PackageFieldAssertion::Scalar(engine_core::ScalarAssertion::Present(
        "present".to_owned(),
    ));
    let mut left = cargo::CargoTomlRequirements::default();
    let _ = left.package_fields.insert("edition".to_owned(), exact);
    let mut right = cargo::CargoTomlRequirements::default();
    let _ = right.package_fields.insert("edition".to_owned(), one);
    let mut third = cargo::CargoTomlRequirements::default();
    let _ = third.package_fields.insert("edition".to_owned(), present);
    let (merged, conflicts) = cargo::CargoTomlRequirements::merge(vec![
        (prov("p1"), left),
        (prov("p2"), right),
        (prov("p3"), third),
    ]);
    assert!(conflicts.is_empty());
    assert!(matches!(
        merged.package_fields["edition"].merged,
        cargo::ResolvedPackageFieldAssertion::Scalar(engine_core::ScalarAssertion::Equals(engine_core::ConfigScalar::Str(ref value), _)) if value == "2021"
    ));
}

#[test]
fn scalar_implication_attributes_only_failed_assertions() {
    let mut allowed = BTreeSet::new();
    let _ = allowed.insert(engine_core::ConfigScalar::Str("2021".to_owned()));
    let _ = allowed.insert(engine_core::ConfigScalar::Str("2024".to_owned()));
    let mut exact_policy = cargo::CargoTomlRequirements::default();
    let _ = exact_policy.package_fields.insert(
        "edition".to_owned(),
        cargo::PackageFieldAssertion::Scalar(engine_core::ScalarAssertion::Equals(
            engine_core::ConfigScalar::Str("2021".to_owned()),
            "exact".to_owned(),
        )),
    );
    let mut oneof_policy = cargo::CargoTomlRequirements::default();
    let _ = oneof_policy.package_fields.insert(
        "edition".to_owned(),
        cargo::PackageFieldAssertion::Scalar(engine_core::ScalarAssertion::OneOf(
            allowed,
            "one".to_owned(),
        )),
    );
    let mut present_policy = cargo::CargoTomlRequirements::default();
    let _ = present_policy.package_fields.insert(
        "edition".to_owned(),
        cargo::PackageFieldAssertion::Scalar(engine_core::ScalarAssertion::Present(
            "present".to_owned(),
        )),
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
        .find(|finding| matches!(finding, engine_core::Finding::Mismatch { message, .. } if message == "exact"))
        .expect("expected an exact-list mismatch finding");
    assert!(matches!(
        mismatch,
        engine_core::Finding::Mismatch { attribution, .. }
            if attribution.iter().any(|p| p.policy == "exact-policy")
                && attribution.iter().all(|p| p.policy != "oneof-policy" && p.policy != "present-policy")
    ));
}

#[test]
fn workspace_inheritance_composes_with_present() {
    let mut left = cargo::CargoTomlRequirements::default();
    let _ = left.package_fields.insert(
        "version".to_owned(),
        cargo::PackageFieldAssertion::Scalar(engine_core::ScalarAssertion::Present(
            "present".to_owned(),
        )),
    );
    let mut right = cargo::CargoTomlRequirements::default();
    let _ = right.package_fields.insert(
        "version".to_owned(),
        cargo::PackageFieldAssertion::InheritsWorkspace("inherit".to_owned()),
    );
    let (merged, conflicts) =
        cargo::CargoTomlRequirements::merge(vec![(prov("p1"), left), (prov("p2"), right)]);
    assert!(conflicts.is_empty());
    assert!(matches!(
        merged.package_fields["version"].merged,
        cargo::ResolvedPackageFieldAssertion::InheritsWorkspace(ref msg) if msg == "inherit"
    ));
}

#[test]
fn cargo_scalar_incompatible_cases_conflict() {
    let mut set = BTreeSet::new();
    let _ = set.insert(engine_core::ConfigScalar::Str("2018".to_owned()));
    let mut left = cargo::CargoTomlRequirements::default();
    let _ = left.package_fields.insert(
        "edition".to_owned(),
        cargo::PackageFieldAssertion::Scalar(engine_core::ScalarAssertion::Equals(
            engine_core::ConfigScalar::Str("2021".to_owned()),
            "2021".to_owned(),
        )),
    );
    let mut right = cargo::CargoTomlRequirements::default();
    let _ = right.package_fields.insert(
        "edition".to_owned(),
        cargo::PackageFieldAssertion::Scalar(engine_core::ScalarAssertion::OneOf(
            set,
            "2018 only".to_owned(),
        )),
    );
    let findings = cargo_findings(vec![(prov("p1"), left), (prov("p2"), right)]);
    assert!(
        findings
            .iter()
            .any(|f| matches!(f, engine_core::Finding::ConflictingRequirements { .. }))
    );
}

#[test]
fn cargo_at_least_version_keeps_strongest_floor() {
    let mut left = cargo::CargoTomlRequirements::default();
    let _ = left.package_fields.insert(
        "rust-version".to_owned(),
        cargo::PackageFieldAssertion::OrderedVersion(engine_core::ScalarAssertion::AtLeast(
            engine_core::DottedVersion::new("1.80"),
            "old".to_owned(),
        )),
    );
    let mut right = cargo::CargoTomlRequirements::default();
    let _ = right.package_fields.insert(
        "rust-version".to_owned(),
        cargo::PackageFieldAssertion::OrderedVersion(engine_core::ScalarAssertion::AtLeast(
            engine_core::DottedVersion::new("1.85"),
            "new".to_owned(),
        )),
    );
    let (merged, conflicts) =
        cargo::CargoTomlRequirements::merge(vec![(prov("p1"), left), (prov("p2"), right)]);
    let cargo::ResolvedPackageFieldAssertion::OrderedVersion(
        engine_core::ScalarAssertion::AtLeast(version, _),
    ) = &merged.package_fields["rust-version"].merged
    else {
        panic!("expected OrderedVersion AtLeast");
    };
    assert!(conflicts.is_empty());
    assert_eq!(version.as_str(), "1.85");
}

#[test]
fn package_lints_inline_preserves_per_lint_attribution() {
    let table = KeyedFixture {
        required: BTreeMap::from([(
            "unwrap_used".to_owned(),
            (
                cargo::LintSetting {
                    level: "deny".to_owned(),
                    priority: None,
                },
                "outer".to_owned(),
            ),
        )]),
        forbidden: BTreeMap::new(),
        closed: None,
    };
    let mut req = cargo::CargoTomlRequirements::default();
    req.package_lints = Some(cargo::PackageLintsAssertion::Inline(BTreeMap::from([(
        "clippy".to_owned(),
        keyed_items(table),
    )])));
    let findings = cargo_findings(vec![(prov("lint-policy"), req)]);
    let contributors = findings
        .iter()
        .filter_map(|f| match f {
            engine_core::Finding::Mismatch { attribution, .. } => Some(attribution),
            _ => None,
        })
        .flatten()
        .collect::<Vec<_>>();
    assert!(contributors.iter().all(|p| p.policy == "lint-policy"));
}
