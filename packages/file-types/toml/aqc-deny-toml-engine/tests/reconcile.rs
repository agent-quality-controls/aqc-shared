#![allow(
    clippy::expect_used,
    reason = "Tests use expect to fail loudly when fixture invariants are broken."
)]
use aqc_toml_engine_core as _;
use serde as _;
use toml_edit as _;

use std::collections::{BTreeMap, BTreeSet};

use aqc_deny_toml_engine::{
    DenyBuildGlobSpec, DenyConfidenceThreshold, DenyDuration, DenyFeatureBanSpec,
    DenyGraphTargetSpec, DenyLintLevel, DenyNonEmptyString, DenyPackageReasonSpec, DenyTable,
    DenyTomlEngine, DenyTomlRequirements, ResolvedDenyTomlRequirements,
};
use aqc_file_engine_core::{
    FileEngine, Finding, ItemRequirements, KeyedItem, ListRequirements, Provenance, ScalarAssertion,
};

#[test]
fn missing_field_is_repaired() {
    let output = reconcile("", scalar_req());
    let expected = expected(&output);
    assert!(
        expected.contains("multiple-versions = \"deny\""),
        "missing scalar field should be written"
    );
}

#[test]
fn valid_drift_is_repaired() {
    let output = reconcile("[bans]\nmultiple-versions = \"allow\"\n", scalar_req());
    let expected = expected(&output);
    assert!(
        expected.contains("multiple-versions = \"deny\""),
        "wrong scalar value should be repaired"
    );
}

#[test]
fn confidence_threshold_writes_float() {
    let output = reconcile(
        "",
        DenyTomlRequirements {
            licenses_confidence_threshold: Some(ScalarAssertion::AtLeast(
                DenyConfidenceThreshold::new("0.8")
                    .expect("test confidence threshold should construct"),
                "minimum confidence".to_owned(),
            )),
            ..DenyTomlRequirements::default()
        },
    );
    let expected = expected(&output);
    assert!(
        expected.contains("confidence-threshold = 0.8"),
        "confidence threshold should be written as a TOML float"
    );
    assert!(
        !expected.contains("confidence-threshold = \"0.8\""),
        "confidence threshold must not be written as a string"
    );
}

#[test]
fn confidence_threshold_at_least_accepts_stricter_value() {
    let output = reconcile(
        "[licenses]\nconfidence-threshold = 0.9\n",
        DenyTomlRequirements {
            licenses_confidence_threshold: Some(ScalarAssertion::AtLeast(
                DenyConfidenceThreshold::new("0.8")
                    .expect("test confidence threshold should construct"),
                "minimum confidence".to_owned(),
            )),
            ..DenyTomlRequirements::default()
        },
    );
    assert!(
        output.findings.is_empty(),
        "stricter confidence threshold should satisfy AtLeast"
    );
}

#[test]
fn confidence_threshold_at_least_repairs_weaker_value() {
    let output = reconcile(
        "[licenses]\nconfidence-threshold = 0.7\n",
        DenyTomlRequirements {
            licenses_confidence_threshold: Some(ScalarAssertion::AtLeast(
                DenyConfidenceThreshold::new("0.8")
                    .expect("test confidence threshold should construct"),
                "minimum confidence".to_owned(),
            )),
            ..DenyTomlRequirements::default()
        },
    );
    let expected = expected(&output);
    assert!(
        expected.contains("confidence-threshold = 0.8"),
        "weaker confidence threshold should be repaired to the minimum"
    );
}

#[test]
fn duration_requires_cargo_deny_shape() {
    assert!(
        DenyDuration::new("90d").is_err(),
        "non cargo-deny duration shape must be rejected"
    );
    assert!(
        DenyDuration::new("P90D").is_ok(),
        "cargo-deny duration shape must be accepted"
    );
}

#[test]
fn wrong_list_member_type() {
    let output = reconcile(
        "[graph]\nfeatures = [1]\n",
        DenyTomlRequirements {
            graph_features: ListRequirements {
                contains: BTreeMap::from([("all".to_owned(), "all".to_owned())]),
                ..ListRequirements::default()
            },
            ..DenyTomlRequirements::default()
        },
    );
    assert!(
        output
            .findings
            .iter()
            .any(|finding| matches!(finding, Finding::Mismatch { key, expected, .. } if key == "graph.features[0]" && expected == "string")),
        "non-string list member should be reported"
    );
}

#[test]
fn empty_list_member() {
    let output = reconcile(
        "[graph]\nfeatures = [\"\"]\n",
        DenyTomlRequirements {
            graph_features: ListRequirements {
                excludes: BTreeMap::from([(String::new(), "empty".to_owned())]),
                ..ListRequirements::default()
            },
            ..DenyTomlRequirements::default()
        },
    );
    assert!(
        output.findings.iter().any(
            |finding| matches!(finding, Finding::Mismatch { key, .. } if key == "graph.features.")
        ),
        "empty list member can be targeted and removed"
    );
}

#[test]
fn wrong_item_shape() {
    let output = reconcile(
        "[bans]\nallow = [1]\n",
        DenyTomlRequirements {
            bans_allow: ItemRequirements {
                required: vec![(
                    DenyPackageReasonSpec::new("serde").expect("package"),
                    "allow serde".to_owned(),
                )],
                ..ItemRequirements::default()
            },
            ..DenyTomlRequirements::default()
        },
    );
    assert!(
        output.findings.iter().any(
            |finding| matches!(finding, Finding::Mismatch { key, .. } if key == "bans.allow[0]")
        ),
        "malformed item should be reported"
    );
}

#[test]
fn duplicate_item_identity() {
    let output = reconcile(
        "[bans]\nallow = [\"serde\", \"serde\"]\n",
        DenyTomlRequirements {
            bans_allow: ItemRequirements {
                required: vec![(
                    DenyPackageReasonSpec::new("serde").expect("package"),
                    "allow serde".to_owned(),
                )],
                ..ItemRequirements::default()
            },
            ..DenyTomlRequirements::default()
        },
    );
    assert!(
        output
            .findings
            .iter()
            .any(|finding| matches!(finding, Finding::InvalidRequirements { key, .. } if key == "bans.allow.serde")),
        "duplicate item identities should be reported"
    );
}

#[test]
fn unused_allowed_org_is_invalid_for_target_schema() {
    let output = reconcile(
        "[sources]\nunused-allowed-org = \"warn\"\n",
        DenyTomlRequirements::default(),
    );
    assert!(
        output
            .findings
            .iter()
            .any(|finding| matches!(finding, Finding::Mismatch { key, .. } if key == "sources.unused-allowed-org")),
        "unsupported local cargo-deny field should be removed"
    );
}

#[test]
fn bans_build_is_valid_when_open() {
    let output = reconcile(
        "[bans.build]\nexecutables = \"deny\"\n",
        DenyTomlRequirements::default(),
    );
    assert!(
        output.findings.is_empty(),
        "valid bans.build should pass through when no requirement closes it"
    );
}

#[test]
fn exact_bans_build_removes_extra() {
    let output = reconcile(
        "[bans.build]\nunknown = true\nexecutables = \"deny\"\n",
        DenyTomlRequirements {
            table_keys: BTreeMap::from([(
                DenyTable::BansBuild,
                ItemRequirements {
                    exact: Some((vec![key("executables")], "exact".to_owned())),
                    ..ItemRequirements::default()
                },
            )]),
            ..DenyTomlRequirements::default()
        },
    );
    let expected = expected(&output);
    assert!(
        !expected.contains("unknown"),
        "exact settings should remove unknown bans.build key"
    );
}

fn key(file_key: &str) -> KeyedItem<()> {
    KeyedItem {
        file_key: file_key.to_owned(),
        value: (),
    }
}

#[test]
fn deprecated_name_repairs_to_crate() {
    let output = reconcile(
        "[bans]\nallow = [{ name = \"serde\", reason = \"ok\" }]\n",
        DenyTomlRequirements {
            bans_allow: ItemRequirements {
                required: vec![(
                    DenyPackageReasonSpec::with_reason("serde", "ok").expect("package"),
                    "allow serde".to_owned(),
                )],
                ..ItemRequirements::default()
            },
            ..DenyTomlRequirements::default()
        },
    );
    let expected = expected(&output);
    assert!(
        expected.contains("crate = \"serde\"") && !expected.contains("name = \"serde\""),
        "deprecated name key should be repaired to crate"
    );
}

#[test]
fn list_output_is_sorted() {
    let output = reconcile(
        "",
        DenyTomlRequirements {
            graph_features: ListRequirements {
                exact: Some((vec!["b".to_owned(), "a".to_owned()], "exact".to_owned())),
                ..ListRequirements::default()
            },
            ..DenyTomlRequirements::default()
        },
    );
    let expected = expected(&output);
    assert!(
        expected.contains("features = [\"a\", \"b\"]"),
        "exact list output should be deterministic"
    );
}

#[test]
fn feature_allow_deny_overlap_is_invalid() {
    let feature =
        DenyNonEmptyString::new("std").expect("test feature name should construct a deny string");
    let err = DenyFeatureBanSpec::new(
        "serde",
        BTreeSet::from([feature.clone()]),
        BTreeSet::from([feature]),
    )
    .expect_err("overlap should be invalid");
    assert!(
        err.to_string().contains("std"),
        "overlap error should name the feature"
    );
}

#[test]
fn item_collections_write_required_items() {
    let output = reconcile(
        "",
        DenyTomlRequirements {
            graph_targets: ItemRequirements {
                required: vec![(
                    DenyGraphTargetSpec::new("x86_64-unknown-linux-gnu").expect("target"),
                    "target".to_owned(),
                )],
                ..ItemRequirements::default()
            },
            bans_build_globs: ItemRequirements {
                required: vec![(
                    DenyBuildGlobSpec::new("**/*.sh").expect("glob"),
                    "glob".to_owned(),
                )],
                ..ItemRequirements::default()
            },
            ..DenyTomlRequirements::default()
        },
    );
    let expected = expected(&output);
    assert!(
        expected.contains("targets = [\"x86_64-unknown-linux-gnu\"]")
            && expected.contains("globs = [\"**/*.sh\"]"),
        "array item helpers should write required items"
    );
}

#[test]
fn explicit_membership_reconciles_every_modeled_deny_table() {
    let current = r"unknown-root = true
[graph]
unknown = true
[output]
unknown = true
[advisories]
unknown = true
[licenses]
unknown = true
[licenses.private]
unknown = true
[bans]
unknown = true
[bans.workspace-dependencies]
unknown = true
[bans.build]
unknown = true
[sources]
unknown = true
[sources.allow-org]
unknown = true
";
    let exact = |keys: &[&str]| ItemRequirements {
        exact: Some((
            keys.iter().map(|key_name| key(key_name)).collect(),
            "exact table".to_owned(),
        )),
        ..ItemRequirements::default()
    };
    let output = reconcile(
        current,
        DenyTomlRequirements {
            table_keys: BTreeMap::from([
                (
                    DenyTable::Root,
                    exact(&[
                        "graph",
                        "output",
                        "advisories",
                        "licenses",
                        "bans",
                        "sources",
                    ]),
                ),
                (DenyTable::Graph, exact(&[])),
                (DenyTable::Output, exact(&[])),
                (DenyTable::Advisories, exact(&[])),
                (DenyTable::Licenses, exact(&["private"])),
                (DenyTable::LicensesPrivate, exact(&[])),
                (DenyTable::Bans, exact(&["workspace-dependencies", "build"])),
                (DenyTable::BansWorkspaceDependencies, exact(&[])),
                (DenyTable::BansBuild, exact(&[])),
                (DenyTable::Sources, exact(&["allow-org"])),
                (DenyTable::SourcesAllowOrg, exact(&[])),
            ]),
            ..DenyTomlRequirements::default()
        },
    );

    assert!(!expected(&output).contains("unknown"));
    assert_eq!(output.findings.len(), 11);
}

#[test]
fn explicit_membership_reports_missing_and_forbidden_deny_keys() {
    let output = reconcile(
        "[graph]\nforbidden = true\n",
        DenyTomlRequirements {
            table_keys: BTreeMap::from([(
                DenyTable::Graph,
                ItemRequirements {
                    required: vec![(key("missing"), "required".to_owned())],
                    forbidden: vec![(key("forbidden"), "forbidden".to_owned())],
                    ..ItemRequirements::default()
                },
            )]),
            ..DenyTomlRequirements::default()
        },
    );

    assert!(output.findings.iter().any(|finding| matches!(
        finding,
        Finding::UnwritableRequiredKey { key, .. } if key == "graph.missing"
    )));
    assert!(output.findings.iter().any(|finding| matches!(
        finding,
        Finding::Mismatch { key, message, .. }
            if key == "graph.forbidden" && message == "forbidden"
    )));
    assert!(!expected(&output).contains("forbidden"));
}

#[test]
fn conflicting_exact_deny_table_keys_fail_merge() {
    let requirement = |key_name: &str| DenyTomlRequirements {
        table_keys: BTreeMap::from([(
            DenyTable::Graph,
            ItemRequirements {
                exact: Some((vec![key(key_name)], key_name.to_owned())),
                ..ItemRequirements::default()
            },
        )]),
        ..DenyTomlRequirements::default()
    };
    let conflicts = DenyTomlRequirements::merge(vec![
        (
            Provenance {
                policy: "one".to_owned(),
            },
            requirement("one"),
        ),
        (
            Provenance {
                policy: "two".to_owned(),
            },
            requirement("two"),
        ),
    ])
    .expect_err("different exact graph key sets must conflict");

    assert!(conflicts.iter().any(|conflict| conflict.key == "graph"));
}

#[test]
fn exact_deny_table_keys_cannot_exclude_a_constructive_value_requirement() {
    let requirement = DenyTomlRequirements {
        bans_multiple_versions: Some(ScalarAssertion::Equals(
            DenyLintLevel::Deny,
            "deny duplicates".to_owned(),
        )),
        table_keys: BTreeMap::from([(
            DenyTable::Bans,
            ItemRequirements {
                exact: Some((Vec::new(), "no bans keys".to_owned())),
                ..ItemRequirements::default()
            },
        )]),
        ..DenyTomlRequirements::default()
    };

    let conflicts = DenyTomlRequirements::merge(vec![(
        Provenance {
            policy: "policy".to_owned(),
        },
        requirement,
    )])
    .expect_err("value and membership requirements must conflict");

    assert!(
        conflicts
            .iter()
            .any(|conflict| conflict.key == "bans.multiple-versions")
    );
}

#[test]
fn exact_parent_keys_cannot_exclude_a_constructive_child_membership() {
    let exact = |keys: &[&str], message: &str| ItemRequirements {
        exact: Some((
            keys.iter().map(|key_name| key(key_name)).collect(),
            message.to_owned(),
        )),
        ..ItemRequirements::default()
    };
    let requirements = DenyTomlRequirements {
        table_keys: BTreeMap::from([
            (DenyTable::Root, exact(&[], "no root tables")),
            (DenyTable::Graph, exact(&["targets"], "graph has targets")),
        ]),
        ..DenyTomlRequirements::default()
    };

    let conflicts = DenyTomlRequirements::merge(vec![(
        Provenance {
            policy: "policy".to_owned(),
        },
        requirements,
    )])
    .expect_err("constructive child membership must require its parent table");

    assert!(
        conflicts
            .iter()
            .any(|conflict| conflict.key == "deny.toml.graph"),
        "unexpected conflicts: {conflicts:?}"
    );
}

#[test]
fn absent_deny_scalar_is_excluded_from_exact_membership() {
    let output = reconcile(
        "[bans]\nmultiple-versions = \"deny\"\n",
        DenyTomlRequirements {
            bans_multiple_versions: Some(ScalarAssertion::Absent("must be absent".to_owned())),
            table_keys: BTreeMap::from([(
                DenyTable::Bans,
                ItemRequirements {
                    exact: Some((Vec::new(), "no bans keys".to_owned())),
                    ..ItemRequirements::default()
                },
            )]),
            ..DenyTomlRequirements::default()
        },
    );

    assert_eq!(output.findings.len(), 1);
    assert!(!expected(&output).contains("multiple-versions"));
}

#[test]
fn rejected_table_key_is_not_also_validated_as_child_content() {
    let output = reconcile(
        "[advisories]\nignore = [\"RUSTSEC-0000-0000\"]\n",
        DenyTomlRequirements {
            advisories_ignore: ItemRequirements {
                exact: Some((Vec::new(), "no ignored advisories".to_owned())),
                ..ItemRequirements::default()
            },
            table_keys: BTreeMap::from([(
                DenyTable::Advisories,
                ItemRequirements {
                    exact: Some((Vec::new(), "no advisories keys".to_owned())),
                    ..ItemRequirements::default()
                },
            )]),
            ..DenyTomlRequirements::default()
        },
    );

    assert!(matches!(
        output.findings.as_slice(),
        [Finding::Mismatch { key, message, .. }]
            if key == "advisories.ignore" && message == "no advisories keys"
    ));
}

#[test]
fn constructive_deny_membership_initializes_to_a_fixed_point() {
    let requirement = DenyTomlRequirements {
        bans_multiple_versions: Some(ScalarAssertion::Equals(
            DenyLintLevel::Deny,
            "deny duplicates".to_owned(),
        )),
        table_keys: BTreeMap::from([(
            DenyTable::Bans,
            ItemRequirements {
                exact: Some((vec![key("multiple-versions")], "only duplicates".to_owned())),
                ..ItemRequirements::default()
            },
        )]),
        ..DenyTomlRequirements::default()
    };
    let resolved = DenyTomlRequirements::merge(vec![(
        Provenance {
            policy: "policy".to_owned(),
        },
        requirement,
    )])
    .expect("requirements must merge");
    let initialized = <DenyTomlEngine as FileEngine<_>>::reconcile(None, &resolved);
    let second =
        <DenyTomlEngine as FileEngine<_>>::reconcile(Some(&initialized.expected_bytes), &resolved);

    assert!(expected(&initialized).contains("multiple-versions = \"deny\""));
    assert!(second.findings.is_empty());
}

#[test]
fn deny_membership_findings_include_all_exact_contributors() {
    let requirement = || DenyTomlRequirements {
        table_keys: BTreeMap::from([(
            DenyTable::Bans,
            ItemRequirements {
                exact: Some((Vec::new(), "no bans keys".to_owned())),
                ..ItemRequirements::default()
            },
        )]),
        ..DenyTomlRequirements::default()
    };
    let resolved = DenyTomlRequirements::merge(vec![
        (
            Provenance {
                policy: "two".to_owned(),
            },
            requirement(),
        ),
        (
            Provenance {
                policy: "one".to_owned(),
            },
            requirement(),
        ),
    ])
    .expect("agreeing requirements must merge");
    let output =
        <DenyTomlEngine as FileEngine<_>>::reconcile(Some(b"[bans]\nunknown = true\n"), &resolved);

    assert!(matches!(
        output.findings.as_slice(),
        [Finding::Mismatch { attribution, .. }]
            if attribution == &[
                Provenance { policy: "one".to_owned() },
                Provenance { policy: "two".to_owned() },
            ]
    ));
}

fn scalar_req() -> DenyTomlRequirements {
    DenyTomlRequirements {
        bans_multiple_versions: Some(ScalarAssertion::Equals(
            DenyLintLevel::Deny,
            "deny duplicates".to_owned(),
        )),
        ..DenyTomlRequirements::default()
    }
}

fn reconcile(current: &str, req: DenyTomlRequirements) -> aqc_file_engine_core::EngineOutput {
    let resolved = DenyTomlRequirements::merge(vec![(
        Provenance {
            policy: "test".to_owned(),
        },
        req,
    )])
    .expect("fixture must merge");
    <DenyTomlEngine as FileEngine<ResolvedDenyTomlRequirements>>::reconcile(
        Some(current.as_bytes()),
        &resolved,
    )
}

fn expected(output: &aqc_file_engine_core::EngineOutput) -> String {
    String::from_utf8(first_bytes(output)).unwrap_or_default()
}

fn first_bytes(output: &aqc_file_engine_core::EngineOutput) -> Vec<u8> {
    output.expected_bytes.clone()
}
