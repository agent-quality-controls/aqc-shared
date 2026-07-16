#![expect(
    clippy::expect_used,
    reason = "Test setup uses expect messages to identify invalid fixtures."
)]

use std::collections::BTreeMap;

use aqc_file_engine_core::{
    FileEngine, Finding, ForbiddenGlobRequirements, ItemRequirements, KeyedItem, ListRequirements,
    Provenance, ScalarAssertion,
};
use aqc_pnpm_workspace_yaml_engine::{
    PnpmOnFail, PnpmPackageSelectorGlob, PnpmReleaseAgeMinutes, PnpmReleaseAgeMinutesError,
    PnpmTrustPolicy, PnpmWorkspaceYamlEngine, PnpmWorkspaceYamlRequirements,
};
use aqc_yaml_engine_core::parse_yaml_mapping;
use globset as _;
use schemars as _;
use serde::Deserialize;
use serde::de::value::{Error as SerdeValueError, U64Deserializer};

fn provenance(policy: &str) -> Provenance {
    Provenance {
        policy: policy.to_owned(),
    }
}

fn forbidden_all() -> ForbiddenGlobRequirements<PnpmPackageSelectorGlob> {
    ForbiddenGlobRequirements {
        globs: vec![(
            PnpmPackageSelectorGlob {
                glob: "**".to_owned(),
            },
            "selectors are forbidden".to_owned(),
        )],
    }
}

fn baseline() -> PnpmWorkspaceYamlRequirements {
    PnpmWorkspaceYamlRequirements {
        strict_peer_dependencies: Some(ScalarAssertion::Equals(true, "strict peers".to_owned())),
        engine_strict: Some(ScalarAssertion::Equals(true, "strict engines".to_owned())),
        minimum_release_age: Some(ScalarAssertion::AtLeast(
            PnpmReleaseAgeMinutes::new(1440).expect("The baseline age must be valid."),
            "release age".to_owned(),
        )),
        minimum_release_age_strict: Some(ScalarAssertion::Equals(
            true,
            "strict release age".to_owned(),
        )),
        minimum_release_age_ignore_missing_time: Some(ScalarAssertion::Equals(
            false,
            "missing time must fail".to_owned(),
        )),
        minimum_release_age_exclude: ListRequirements::default(),
        forbidden_minimum_release_age_exclude_globs: forbidden_all(),
        trust_policy: Some(ScalarAssertion::Equals(
            PnpmTrustPolicy::NoDowngrade,
            "trust policy".to_owned(),
        )),
        trust_lockfile: Some(ScalarAssertion::Equals(false, "lockfile trust".to_owned())),
        trust_policy_ignore_after: Some(ScalarAssertion::Absent(
            "trust cutoff forbidden".to_owned(),
        )),
        trust_policy_exclude: ListRequirements::default(),
        forbidden_trust_policy_exclude_globs: forbidden_all(),
        block_exotic_subdeps: Some(ScalarAssertion::Equals(true, "block exotic".to_owned())),
        pm_on_fail: Some(ScalarAssertion::Equals(
            PnpmOnFail::Error,
            "version mismatch must fail".to_owned(),
        )),
        strict_dep_builds: Some(ScalarAssertion::Equals(true, "strict builds".to_owned())),
        dangerously_allow_all_builds: Some(ScalarAssertion::Equals(
            false,
            "allow all forbidden".to_owned(),
        )),
        allow_builds: ItemRequirements::default(),
        forbidden_allowed_build_package_globs: forbidden_all(),
        exact_settings: None,
    }
}

#[test]
fn release_age_enforces_javascript_safe_integer() {
    assert_eq!(
        PnpmReleaseAgeMinutes::new(0).map(|value| value.get()),
        Ok(0)
    );
    assert_eq!(
        PnpmReleaseAgeMinutes::new(9_007_199_254_740_991).map(|value| value.get()),
        Ok(9_007_199_254_740_991)
    );
    assert_eq!(
        PnpmReleaseAgeMinutes::new(9_007_199_254_740_992),
        Err(PnpmReleaseAgeMinutesError::ExceedsJavaScriptSafeInteger)
    );
}

#[test]
fn inherited_forbidden_collections_are_overridden_by_empty_direct_collections() {
    let requirements = PnpmWorkspaceYamlRequirements {
        forbidden_minimum_release_age_exclude_globs: forbidden_all(),
        forbidden_allowed_build_package_globs: forbidden_all(),
        ..PnpmWorkspaceYamlRequirements::default()
    };
    let resolved = PnpmWorkspaceYamlRequirements::merge(vec![(provenance("policy"), requirements)])
        .expect("Requirements must merge.");
    let bytes = b"defaults: &defaults\n  minimumReleaseAgeExclude: [react]\n  allowBuilds: {esbuild: true}\n<<: *defaults\n";
    let output = <PnpmWorkspaceYamlEngine as FileEngine<_>>::reconcile(Some(bytes), &resolved);
    let parsed = parse_yaml_mapping(Some(&output.expected_bytes), "YAML document")
        .expect("Reconciled YAML must parse.");
    assert_eq!(
        parsed.field("minimumReleaseAgeExclude"),
        Ok(Some(aqc_yaml_engine_core::YamlFieldValue::StringSequence(
            Vec::new()
        )))
    );
    assert_eq!(
        parsed.field("allowBuilds"),
        Ok(Some(
            aqc_yaml_engine_core::YamlFieldValue::StringBooleanMapping(BTreeMap::new())
        ))
    );
    let fixed = <PnpmWorkspaceYamlEngine as FileEngine<_>>::reconcile(
        Some(&output.expected_bytes),
        &resolved,
    );
    assert!(fixed.findings.is_empty());
    assert_eq!(fixed.expected_bytes, output.expected_bytes);
}

#[test]
fn absent_scalar_does_not_remove_a_direct_override_that_would_expose_inheritance() {
    let requirements = PnpmWorkspaceYamlRequirements {
        trust_policy_ignore_after: Some(ScalarAssertion::Absent("forbidden".to_owned())),
        ..PnpmWorkspaceYamlRequirements::default()
    };
    let resolved = PnpmWorkspaceYamlRequirements::merge(vec![(provenance("policy"), requirements)])
        .expect("Requirements must merge.");
    let bytes = b"defaults: &defaults\n  trustPolicyIgnoreAfter: inherited\n<<: *defaults\ntrustPolicyIgnoreAfter: direct\n";
    let output = <PnpmWorkspaceYamlEngine as FileEngine<_>>::reconcile(Some(bytes), &resolved);
    assert_eq!(output.expected_bytes, bytes);
    assert_eq!(output.findings.len(), 1);
}

#[test]
fn release_age_serde_integer_enforces_javascript_safe_integer() {
    let deserialize =
        |value| PnpmReleaseAgeMinutes::deserialize(U64Deserializer::<SerdeValueError>::new(value));

    assert_eq!(
        deserialize(9_007_199_254_740_991).map(|value| value.get()),
        Ok(9_007_199_254_740_991)
    );
    assert!(deserialize(9_007_199_254_740_992).is_err());
}

#[test]
fn missing_file_renders_complete_deterministic_baseline() {
    let resolved = PnpmWorkspaceYamlRequirements::merge(vec![(provenance("policy"), baseline())])
        .expect("The baseline must merge.");
    let output = <PnpmWorkspaceYamlEngine as FileEngine<_>>::reconcile(None, &resolved);
    let yaml =
        String::from_utf8(output.expected_bytes.clone()).expect("Generated YAML must be UTF-8.");
    assert_eq!(
        yaml,
        "strictPeerDependencies: true\nengineStrict: true\nminimumReleaseAge: 1440\nminimumReleaseAgeStrict: true\nminimumReleaseAgeIgnoreMissingTime: false\ntrustPolicy: no-downgrade\ntrustLockfile: false\nblockExoticSubdeps: true\npmOnFail: error\nstrictDepBuilds: true\ndangerouslyAllowAllBuilds: false\n"
    );
    let clean = <PnpmWorkspaceYamlEngine as FileEngine<_>>::reconcile(
        Some(&output.expected_bytes),
        &resolved,
    );
    assert!(clean.findings.is_empty());
    assert_eq!(clean.expected_bytes, output.expected_bytes);
}

#[test]
fn reversing_agreeing_policy_order_preserves_generated_bytes() {
    let first = PnpmWorkspaceYamlRequirements::merge(vec![
        (provenance("alpha"), baseline()),
        (provenance("beta"), baseline()),
    ])
    .expect("Agreeing requirements must merge.");
    let reversed = PnpmWorkspaceYamlRequirements::merge(vec![
        (provenance("beta"), baseline()),
        (provenance("alpha"), baseline()),
    ])
    .expect("Agreeing requirements must merge.");
    let first_output = <PnpmWorkspaceYamlEngine as FileEngine<_>>::reconcile(None, &first);
    let reversed_output = <PnpmWorkspaceYamlEngine as FileEngine<_>>::reconcile(None, &reversed);
    assert_eq!(first_output.expected_bytes, reversed_output.expected_bytes);
    assert_eq!(
        format!("{:?}", first_output.findings),
        format!("{:?}", reversed_output.findings)
    );
}

#[test]
fn scalar_disagreement_preserves_key_reason_and_contributors() {
    let requirement = |value| PnpmWorkspaceYamlRequirements {
        strict_peer_dependencies: Some(ScalarAssertion::Equals(value, "strict peers".to_owned())),
        ..PnpmWorkspaceYamlRequirements::default()
    };
    let conflicts = PnpmWorkspaceYamlRequirements::merge(vec![
        (provenance("enabled"), requirement(true)),
        (provenance("disabled"), requirement(false)),
    ])
    .expect_err("Disagreeing scalar requirements must conflict.");
    assert_eq!(conflicts.len(), 1);
    let conflict = conflicts.first().expect("One conflict must be present.");
    assert_eq!(conflict.key, "strictPeerDependencies");
    assert_eq!(conflict.reason, "scalar-disagree");
    assert_eq!(
        conflict
            .contributors
            .iter()
            .map(|(source, _)| source.policy.as_str())
            .collect::<Vec<_>>(),
        ["disabled", "enabled"]
    );
}

#[test]
fn clean_existing_bytes_are_preserved_exactly() {
    let requirements = PnpmWorkspaceYamlRequirements {
        strict_peer_dependencies: Some(ScalarAssertion::Equals(true, "strict peers".to_owned())),
        ..PnpmWorkspaceYamlRequirements::default()
    };
    let resolved = PnpmWorkspaceYamlRequirements::merge(vec![(provenance("policy"), requirements)])
        .expect("Requirements must merge.");
    let bytes = b"# keep\nstrictPeerDependencies: true # keep inline\n";
    let output = <PnpmWorkspaceYamlEngine as FileEngine<_>>::reconcile(Some(bytes), &resolved);
    assert!(output.findings.is_empty());
    assert_eq!(output.expected_bytes, bytes);
}

#[test]
fn inherited_values_validate_and_direct_values_take_precedence() {
    let requirements = PnpmWorkspaceYamlRequirements {
        strict_peer_dependencies: Some(ScalarAssertion::Equals(true, "strict peers".to_owned())),
        ..PnpmWorkspaceYamlRequirements::default()
    };
    let resolved = PnpmWorkspaceYamlRequirements::merge(vec![(provenance("policy"), requirements)])
        .expect("Requirements must merge.");
    let inherited = b"defaults: &defaults\n  strictPeerDependencies: true\n<<: *defaults\n";
    assert!(
        <PnpmWorkspaceYamlEngine as FileEngine<_>>::reconcile(Some(inherited), &resolved)
            .findings
            .is_empty()
    );
    let direct = b"defaults: &defaults\n  strictPeerDependencies: true\n<<: *defaults\nstrictPeerDependencies: false\n";
    let output = <PnpmWorkspaceYamlEngine as FileEngine<_>>::reconcile(Some(direct), &resolved);
    assert_eq!(output.findings.len(), 1);
    assert!(
        String::from_utf8(output.expected_bytes)
            .expect("Expected YAML must be UTF-8.")
            .contains("strictPeerDependencies: true")
    );
}

#[test]
fn forbidden_selectors_receive_item_selectors() {
    let requirements = PnpmWorkspaceYamlRequirements {
        forbidden_minimum_release_age_exclude_globs: forbidden_all(),
        forbidden_trust_policy_exclude_globs: forbidden_all(),
        forbidden_allowed_build_package_globs: forbidden_all(),
        ..PnpmWorkspaceYamlRequirements::default()
    };
    let resolved = PnpmWorkspaceYamlRequirements::merge(vec![(provenance("policy"), requirements)])
        .expect("Requirements must merge.");
    let yaml = b"minimumReleaseAgeExclude:\n- react\ntrustPolicyExclude:\n- '@scope/pkg'\nallowBuilds: {\"esbuild\": true, \"blocked\": false}\n";
    let output = <PnpmWorkspaceYamlEngine as FileEngine<_>>::reconcile(Some(yaml), &resolved);
    let selectors = output
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
        .collect::<Vec<_>>();
    assert_eq!(selectors, vec!["react", "@scope/pkg", "esbuild"]);
    let expected = String::from_utf8(output.expected_bytes.clone()).expect("YAML must be UTF-8.");
    assert!(!expected.contains("react"));
    assert!(!expected.contains("@scope/pkg"));
    assert!(!expected.contains("esbuild"));
    let parsed = parse_yaml_mapping(Some(expected.as_bytes()), "pnpm-workspace.yaml")
        .expect("Expected bytes must remain valid YAML.");
    assert_eq!(
        parsed
            .field("allowBuilds")
            .expect("allowBuilds must be readable"),
        Some(aqc_yaml_engine_core::YamlFieldValue::StringBooleanMapping(
            BTreeMap::from([("blocked".to_owned(), false)]),
        )),
        "reconciled YAML:\n{expected}",
    );
    let fixed_point = <PnpmWorkspaceYamlEngine as FileEngine<_>>::reconcile(
        Some(&output.expected_bytes),
        &resolved,
    );
    assert!(
        fixed_point.findings.is_empty(),
        "Reconciled exact output must be stable: {:?}",
        fixed_point.findings,
    );
}

#[test]
fn duplicate_glob_contributors_produce_one_attributed_item_finding() {
    let requirement = || PnpmWorkspaceYamlRequirements {
        forbidden_trust_policy_exclude_globs: forbidden_all(),
        ..PnpmWorkspaceYamlRequirements::default()
    };
    let resolved = PnpmWorkspaceYamlRequirements::merge(vec![
        (provenance("one"), requirement()),
        (provenance("two"), requirement()),
    ])
    .expect("Agreeing globs must merge.");
    let output = <PnpmWorkspaceYamlEngine as FileEngine<_>>::reconcile(
        Some(b"trustPolicyExclude: [react]\n"),
        &resolved,
    );

    assert!(matches!(
        output.findings.as_slice(),
        [Finding::Mismatch { selector: Some(selector), attribution, .. }]
            if selector == "react" && attribution.len() == 2
    ));
}

#[test]
fn required_selector_conflicts_with_matching_forbidden_glob() {
    let requirements = PnpmWorkspaceYamlRequirements {
        minimum_release_age_exclude: ListRequirements {
            contains: [("react".to_owned(), "required exception".to_owned())].into(),
            ..ListRequirements::default()
        },
        forbidden_minimum_release_age_exclude_globs: forbidden_all(),
        ..PnpmWorkspaceYamlRequirements::default()
    };
    assert!(
        PnpmWorkspaceYamlRequirements::merge(vec![(provenance("policy"), requirements)]).is_err()
    );
}

#[test]
fn exact_selector_conflict_preserves_key_reason_and_contributors() {
    let exact = PnpmWorkspaceYamlRequirements {
        minimum_release_age_exclude: ListRequirements {
            exact: Some((vec!["react".to_owned()], "only react".to_owned())),
            ..ListRequirements::default()
        },
        ..PnpmWorkspaceYamlRequirements::default()
    };
    let forbidden = PnpmWorkspaceYamlRequirements {
        forbidden_minimum_release_age_exclude_globs: forbidden_all(),
        ..PnpmWorkspaceYamlRequirements::default()
    };
    let conflicts = PnpmWorkspaceYamlRequirements::merge(vec![
        (provenance("exact"), exact),
        (provenance("forbidden"), forbidden),
    ])
    .expect_err("An exact required selector must conflict with a matching forbidden glob.");
    assert_eq!(conflicts.len(), 1);
    let conflict = conflicts.first().expect("One conflict must be present.");
    assert_eq!(conflict.key, "minimumReleaseAgeExclude.react");
    assert_eq!(conflict.reason, "required-forbidden-glob");
    assert_eq!(
        conflict
            .contributors
            .iter()
            .map(|(source, _)| source.policy.as_str())
            .collect::<Vec<_>>(),
        ["exact", "forbidden"]
    );
}

#[test]
fn contains_and_exact_for_the_same_forbidden_selector_produce_one_conflict() {
    let required = PnpmWorkspaceYamlRequirements {
        minimum_release_age_exclude: ListRequirements {
            contains: [("react".to_owned(), "contains react".to_owned())].into(),
            exact: Some((vec!["react".to_owned()], "exact react".to_owned())),
            ..ListRequirements::default()
        },
        ..PnpmWorkspaceYamlRequirements::default()
    };
    let forbidden = PnpmWorkspaceYamlRequirements {
        forbidden_minimum_release_age_exclude_globs: forbidden_all(),
        ..PnpmWorkspaceYamlRequirements::default()
    };
    let conflicts = PnpmWorkspaceYamlRequirements::merge(vec![
        (provenance("required"), required),
        (provenance("forbidden"), forbidden),
    ])
    .expect_err("The selector must conflict.");
    assert_eq!(conflicts.len(), 1);
    assert_eq!(
        conflicts
            .first()
            .expect("One conflict must be present.")
            .key,
        "minimumReleaseAgeExclude.react"
    );
}

#[test]
fn duplicate_selector_values_produce_one_finding() {
    let requirements = PnpmWorkspaceYamlRequirements {
        forbidden_trust_policy_exclude_globs: forbidden_all(),
        ..PnpmWorkspaceYamlRequirements::default()
    };
    let resolved = PnpmWorkspaceYamlRequirements::merge(vec![(provenance("policy"), requirements)])
        .expect("Requirements must merge.");
    let output = <PnpmWorkspaceYamlEngine as FileEngine<_>>::reconcile(
        Some(b"trustPolicyExclude: [react, react]\n"),
        &resolved,
    );
    assert_eq!(output.findings.len(), 1);
    assert!(matches!(
        output.findings.first().expect("One finding must be present."),
        Finding::Mismatch { selector: Some(selector), .. } if selector == "react"
    ));
}

#[test]
fn exact_list_differences_are_member_specific_order_aware_and_attributed() {
    let exact = PnpmWorkspaceYamlRequirements {
        trust_policy_exclude: ListRequirements {
            exact: Some((
                vec!["a".to_owned(), "a".to_owned(), "b".to_owned()],
                "exact".to_owned(),
            )),
            ..ListRequirements::default()
        },
        ..PnpmWorkspaceYamlRequirements::default()
    };
    let resolved = PnpmWorkspaceYamlRequirements::merge(vec![(provenance("exact"), exact)])
        .expect("Exact requirements must merge.");
    let membership = <PnpmWorkspaceYamlEngine as FileEngine<_>>::reconcile(
        Some(b"trustPolicyExclude: [a, c]\n"),
        &resolved,
    );
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
        .collect::<Vec<_>>();
    assert_eq!(selectors, ["a", "b", "c"]);
    assert!(membership.findings.iter().all(|finding| matches!(
        finding,
        Finding::Mismatch { attribution, .. } if attribution == &vec![provenance("exact")]
    )));

    let order = <PnpmWorkspaceYamlEngine as FileEngine<_>>::reconcile(
        Some(b"trustPolicyExclude: [b, a, a]\n"),
        &resolved,
    );
    assert!(matches!(
        order.findings.as_slice(),
        [Finding::Mismatch { selector: None, .. }]
    ));

    let empty_exact = PnpmWorkspaceYamlRequirements {
        trust_policy_exclude: ListRequirements {
            exact: Some((Vec::new(), "empty".to_owned())),
            ..ListRequirements::default()
        },
        ..PnpmWorkspaceYamlRequirements::default()
    };
    let empty_resolved =
        PnpmWorkspaceYamlRequirements::merge(vec![(provenance("empty"), empty_exact)])
            .expect("Empty exact requirements must merge.");
    let missing =
        <PnpmWorkspaceYamlEngine as FileEngine<_>>::reconcile(Some(b"{}\n"), &empty_resolved);
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
            .expect("YAML is UTF-8")
            .contains("trustPolicyExclude: []")
    );
}

#[test]
fn compatible_exact_member_assertions_share_selector_identity() {
    let exact = PnpmWorkspaceYamlRequirements {
        trust_policy_exclude: ListRequirements {
            exact: Some((vec!["react".to_owned()], "exact".to_owned())),
            ..ListRequirements::default()
        },
        ..PnpmWorkspaceYamlRequirements::default()
    };
    let contains = PnpmWorkspaceYamlRequirements {
        trust_policy_exclude: ListRequirements {
            contains: BTreeMap::from([("react".to_owned(), "contains".to_owned())]),
            ..ListRequirements::default()
        },
        ..PnpmWorkspaceYamlRequirements::default()
    };
    let excludes = PnpmWorkspaceYamlRequirements {
        trust_policy_exclude: ListRequirements {
            excludes: BTreeMap::from([("blocked".to_owned(), "excludes".to_owned())]),
            ..ListRequirements::default()
        },
        ..PnpmWorkspaceYamlRequirements::default()
    };
    let resolved = PnpmWorkspaceYamlRequirements::merge(vec![
        (provenance("exact-policy"), exact),
        (provenance("contains-policy"), contains),
        (provenance("excludes-policy"), excludes),
    ])
    .expect("Compatible requirements must merge.");
    let output = <PnpmWorkspaceYamlEngine as FileEngine<_>>::reconcile(
        Some(b"trustPolicyExclude: []\n"),
        &resolved,
    );
    assert_eq!(output.findings.len(), 2);
    for (message, policy) in [("exact", "exact-policy"), ("contains", "contains-policy")] {
        assert!(output.findings.iter().any(|finding| matches!(
            finding,
            Finding::Mismatch { key, selector: Some(selector), message: found_message, attribution, .. }
                if key == "trustPolicyExclude"
                    && selector == "react"
                    && found_message == message
                    && attribution == &vec![provenance(policy)]
        )));
    }

    let blocked_output = <PnpmWorkspaceYamlEngine as FileEngine<_>>::reconcile(
        Some(b"trustPolicyExclude:\n  - react\n  - blocked\n"),
        &resolved,
    );
    assert_eq!(blocked_output.findings.len(), 2);
    for (message, policy) in [("exact", "exact-policy"), ("excludes", "excludes-policy")] {
        assert!(blocked_output.findings.iter().any(|finding| matches!(
            finding,
            Finding::Mismatch { key, selector: Some(selector), message: found_message, attribution, .. }
                if key == "trustPolicyExclude"
                    && selector == "blocked"
                    && found_message == message
                    && attribution == &vec![provenance(policy)]
        )));
    }
}

#[test]
fn false_allow_build_entry_does_not_conflict_but_true_entry_does() {
    let requirement = |value| PnpmWorkspaceYamlRequirements {
        allow_builds: ItemRequirements {
            required: vec![(
                KeyedItem {
                    file_key: "pkg".to_owned(),
                    value,
                },
                "required build setting".to_owned(),
            )],
            ..ItemRequirements::default()
        },
        forbidden_allowed_build_package_globs: forbidden_all(),
        ..PnpmWorkspaceYamlRequirements::default()
    };
    assert!(
        PnpmWorkspaceYamlRequirements::merge(vec![(provenance("policy"), requirement(false))])
            .is_ok()
    );
    assert!(
        PnpmWorkspaceYamlRequirements::merge(vec![(provenance("policy"), requirement(true))])
            .is_err()
    );
}

#[test]
fn exact_true_allow_build_conflict_preserves_key_reason_and_contributors() {
    let exact = PnpmWorkspaceYamlRequirements {
        allow_builds: ItemRequirements {
            exact: Some((
                vec![KeyedItem {
                    file_key: "esbuild".to_owned(),
                    value: true,
                }],
                "only esbuild".to_owned(),
            )),
            ..ItemRequirements::default()
        },
        ..PnpmWorkspaceYamlRequirements::default()
    };
    let forbidden = PnpmWorkspaceYamlRequirements {
        forbidden_allowed_build_package_globs: forbidden_all(),
        ..PnpmWorkspaceYamlRequirements::default()
    };
    let conflicts = PnpmWorkspaceYamlRequirements::merge(vec![
        (provenance("exact"), exact),
        (provenance("forbidden"), forbidden),
    ])
    .expect_err("An exact true build permission must conflict with a forbidden glob.");
    assert_eq!(conflicts.len(), 1);
    let conflict = conflicts.first().expect("One conflict must be present.");
    assert_eq!(conflict.key, "allowBuilds.esbuild");
    assert_eq!(conflict.reason, "required-forbidden-glob");
    assert_eq!(
        conflict
            .contributors
            .iter()
            .map(|(source, _)| source.policy.as_str())
            .collect::<Vec<_>>(),
        ["exact", "forbidden"]
    );
}

#[test]
fn compatible_release_age_floors_choose_the_strongest_floor() {
    let requirement = |age| PnpmWorkspaceYamlRequirements {
        minimum_release_age: Some(ScalarAssertion::AtLeast(
            PnpmReleaseAgeMinutes::new(age).expect("The age must be valid."),
            "floor".to_owned(),
        )),
        ..PnpmWorkspaceYamlRequirements::default()
    };
    let resolved = PnpmWorkspaceYamlRequirements::merge(vec![
        (provenance("one"), requirement(60)),
        (provenance("two"), requirement(1440)),
    ])
    .expect("Compatible floors must merge.");
    let merged = resolved
        .minimum_release_age()
        .expect("The floor must be resolved.");
    assert!(matches!(&merged.merged, ScalarAssertion::AtLeast(value, _) if value.get() == 1440));
}

#[test]
fn invalid_glob_reports_an_attributed_requirement_finding() {
    let requirements = PnpmWorkspaceYamlRequirements {
        forbidden_trust_policy_exclude_globs: ForbiddenGlobRequirements {
            globs: vec![(
                PnpmPackageSelectorGlob {
                    glob: "[".to_owned(),
                },
                "invalid selector".to_owned(),
            )],
        },
        ..PnpmWorkspaceYamlRequirements::default()
    };
    let resolved = PnpmWorkspaceYamlRequirements::merge(vec![(provenance("policy"), requirements)])
        .expect("Invalid syntax is reported during reconciliation.");
    let output = <PnpmWorkspaceYamlEngine as FileEngine<_>>::reconcile(None, &resolved);
    assert!(
        matches!(output.findings.as_slice(), [Finding::InvalidRequirements { key, contributors, .. }] if key == "trustPolicyExclude" && contributors.len() == 1)
    );
}

#[test]
fn exact_settings_reports_unrepresented_top_level_keys() {
    let requirements = PnpmWorkspaceYamlRequirements {
        exact_settings: Some("only represented pnpm settings are allowed".to_owned()),
        ..PnpmWorkspaceYamlRequirements::default()
    };
    let resolved = PnpmWorkspaceYamlRequirements::merge(vec![(provenance("policy"), requirements)])
        .expect("Exact settings must merge.");
    let output = <PnpmWorkspaceYamlEngine as FileEngine<_>>::reconcile(
        Some(b"packages:\n- app\n"),
        &resolved,
    );
    assert!(matches!(
        output.findings.as_slice(),
        [Finding::Mismatch { key, selector: None, .. }] if key == "packages"
    ));
    let expected = parse_yaml_mapping(Some(&output.expected_bytes), "pnpm-workspace.yaml")
        .expect("Expected bytes must remain valid YAML.");
    assert!(expected.direct_keys().is_empty());
    assert!(
        <PnpmWorkspaceYamlEngine as FileEngine<_>>::reconcile(
            Some(&output.expected_bytes),
            &resolved,
        )
        .findings
        .is_empty()
    );
}

#[test]
fn exact_settings_authorizes_only_fields_present_in_the_resolved_requirement() {
    let requirements = PnpmWorkspaceYamlRequirements {
        strict_peer_dependencies: Some(ScalarAssertion::Equals(true, "strict peers".to_owned())),
        exact_settings: Some("only represented pnpm settings are allowed".to_owned()),
        ..PnpmWorkspaceYamlRequirements::default()
    };
    let resolved = PnpmWorkspaceYamlRequirements::merge(vec![(provenance("policy"), requirements)])
        .expect("Exact settings must merge.");
    let output = <PnpmWorkspaceYamlEngine as FileEngine<_>>::reconcile(
        Some(b"strictPeerDependencies: true\nengineStrict: true\n"),
        &resolved,
    );
    assert!(
        output.findings.iter().any(
            |finding| matches!(finding, Finding::Mismatch { key, .. } if key == "engineStrict")
        )
    );
    assert!(!output.findings.iter().any(
        |finding| matches!(finding, Finding::Mismatch { key, .. } if key == "strictPeerDependencies")
    ));
}

#[test]
fn exact_only_does_not_validate_unrequested_collection_shapes() {
    let requirements = PnpmWorkspaceYamlRequirements {
        exact_settings: Some("only represented pnpm settings are allowed".to_owned()),
        ..PnpmWorkspaceYamlRequirements::default()
    };
    let resolved = PnpmWorkspaceYamlRequirements::merge(vec![(provenance("policy"), requirements)])
        .expect("Exact settings must merge.");
    let output = <PnpmWorkspaceYamlEngine as FileEngine<_>>::reconcile(
        Some(b"allowBuilds: [react]\n"),
        &resolved,
    );

    assert!(matches!(
        output.findings.as_slice(),
        [Finding::Mismatch { key, attribution, .. }]
            if key == "allowBuilds" && attribution == &[provenance("policy")]
    ));
}

#[test]
fn exact_setting_attribution_is_independent_of_registration_order() {
    let requirement = |message: &str| PnpmWorkspaceYamlRequirements {
        exact_settings: Some(message.to_owned()),
        ..PnpmWorkspaceYamlRequirements::default()
    };
    let first = PnpmWorkspaceYamlRequirements::merge(vec![
        (provenance("beta"), requirement("beta message")),
        (provenance("alpha"), requirement("alpha message")),
    ])
    .expect("Exact settings must merge.");
    let reversed = PnpmWorkspaceYamlRequirements::merge(vec![
        (provenance("alpha"), requirement("alpha message")),
        (provenance("beta"), requirement("beta message")),
    ])
    .expect("Exact settings must merge.");

    let bytes = b"packages: [app]\n";
    let first_output = <PnpmWorkspaceYamlEngine as FileEngine<_>>::reconcile(Some(bytes), &first);
    let reversed_output =
        <PnpmWorkspaceYamlEngine as FileEngine<_>>::reconcile(Some(bytes), &reversed);
    assert_eq!(
        format!("{:?}", first_output.findings),
        format!("{:?}", reversed_output.findings)
    );
    assert!(matches!(
        first_output.findings.as_slice(),
        [Finding::Mismatch { message, attribution, .. }]
            if message == "alpha message"
                && attribution == &[provenance("alpha"), provenance("beta")]
    ));
}

#[test]
fn exact_settings_reports_effective_keys_without_rewriting_anchor_sources() {
    let requirements = PnpmWorkspaceYamlRequirements {
        exact_settings: Some("only represented pnpm settings are allowed".to_owned()),
        ..PnpmWorkspaceYamlRequirements::default()
    };
    let resolved = PnpmWorkspaceYamlRequirements::merge(vec![
        (provenance("one"), requirements.clone()),
        (provenance("two"), requirements),
    ])
    .expect("Exact settings must merge.");
    let bytes = b"defaults: &defaults\n  inherited: true\n<<: *defaults\n";
    let output = <PnpmWorkspaceYamlEngine as FileEngine<_>>::reconcile(Some(bytes), &resolved);

    assert!(output.findings.iter().any(|finding| {
        matches!(finding, Finding::Mismatch { key, attribution, .. } if key == "inherited" && attribution.len() == 2)
    }));
    assert!(
        !output
            .findings
            .iter()
            .any(|finding| { matches!(finding, Finding::Mismatch { key, .. } if key == "<<") })
    );
    assert_eq!(output.expected_bytes, bytes);
}

#[test]
fn typed_present_rejects_a_wrong_shape() {
    let requirements = PnpmWorkspaceYamlRequirements {
        strict_peer_dependencies: Some(ScalarAssertion::Present("must be boolean".to_owned())),
        ..PnpmWorkspaceYamlRequirements::default()
    };
    let resolved = PnpmWorkspaceYamlRequirements::merge(vec![(provenance("policy"), requirements)])
        .expect("Present must merge.");
    let output = <PnpmWorkspaceYamlEngine as FileEngine<_>>::reconcile(
        Some(b"strictPeerDependencies: wrong\n"),
        &resolved,
    );

    assert_eq!(output.findings.len(), 1);
}
