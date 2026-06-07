//! Behavior probes for `CargoTomlRequirement::merge` (the merge phase).
//!
//! The module is named `merge` so the test paths read `merge::disjoint`,
//! `merge::conflict`, `merge::identical` (the manifest's verification names).

use toml_edit as _;

mod merge {
    use aqc_cargo_toml_engine::{
        CargoTomlRequirement, DependencyKind, DependencyScope, DependencySetAssertion,
        DependencySpec, LintLevelsAssertion, PackageFieldAssertion, PackageLintsAssertion,
    };
    use aqc_file_engine_core::{ConfigScalar, Provenance};
    use std::collections::BTreeMap;

    /// A requirement asserting one `[package].<field>` from one policy.
    fn field_req(
        policy: &str,
        field: &str,
        assertion: PackageFieldAssertion,
    ) -> CargoTomlRequirement {
        let mut r = CargoTomlRequirement::default();
        let _ = r.package_fields.insert(
            field.to_owned(),
            vec![(
                Provenance {
                    policy: policy.to_owned(),
                },
                assertion,
            )],
        );
        r
    }

    /// A requirement asserting `[package].edition == <edition>` from one policy.
    fn edition_req(policy: &str, edition: &str, msg: &str) -> CargoTomlRequirement {
        field_req(
            policy,
            "edition",
            PackageFieldAssertion::Equals(ConfigScalar::Str(edition.to_owned()), msg.to_owned()),
        )
    }

    /// A requirement asserting one `[dependencies]` entry from one policy.
    fn dep_req(policy: &str, name: &str, version: &str) -> CargoTomlRequirement {
        let mut entries = BTreeMap::new();
        let _ = entries.insert(
            name.to_owned(),
            (
                DependencySpec {
                    version: Some(version.to_owned()),
                    ..DependencySpec::default()
                },
                "fixture: pinned dep".to_owned(),
            ),
        );
        let mut r = CargoTomlRequirement::default();
        let _ = r.dependencies.insert(
            DependencyScope {
                kind: DependencyKind::Normal,
                target: None,
            },
            vec![(
                Provenance {
                    policy: policy.to_owned(),
                },
                DependencySetAssertion::Contains(entries),
            )],
        );
        r
    }

    #[test]
    fn disjoint() {
        let a = edition_req("p1", "2021", "m1");
        let b = field_req(
            "p2",
            "rust-version",
            PackageFieldAssertion::AtLeastVersion("1.85".to_owned(), "m2".to_owned()),
        );
        let (merged, conflicts) = CargoTomlRequirement::merge(&[&a, &b]);
        assert!(conflicts.is_empty(), "disjoint keys must not conflict");
        assert!(
            merged.package_fields.contains_key("edition"),
            "edition survives the merge"
        );
        assert!(
            merged.package_fields.contains_key("rust-version"),
            "rust-version survives the merge"
        );
    }

    #[test]
    fn identical() {
        // Identical values agree (dedup for free) -- and the policy message is
        // not part of the agreement, so differing messages also agree.
        let a = edition_req("p1", "2021", "reason one");
        let b = edition_req("p2", "2021", "reason two");
        let (merged, conflicts) = CargoTomlRequirement::merge(&[&a, &b]);
        assert!(conflicts.is_empty(), "identical semantic values agree");
        assert!(
            merged.package_fields.contains_key("edition"),
            "the agreed edition survives"
        );
        let contribution_count = merged.package_fields.get("edition").map_or(0, Vec::len);
        assert_eq!(contribution_count, 2, "both provenances are kept");
    }

    #[test]
    fn conflict() {
        let a = edition_req("p1", "2021", "m1");
        let b = edition_req("p2", "2018", "m2");
        let (merged, conflicts) = CargoTomlRequirement::merge(&[&a, &b]);
        assert_eq!(
            conflicts.len(),
            1,
            "one per-key conflict for the disagreement"
        );
        assert!(
            conflicts.iter().any(|c| c.key == "[package].edition"),
            "the conflict names the disagreeing in-file key"
        );
        assert!(
            conflicts.iter().all(|c| c.contributors.len() == 2),
            "both disagreeing policies are named"
        );
        assert!(
            !merged.package_fields.contains_key("edition"),
            "the conflicting field is dropped, not written"
        );
    }

    #[test]
    fn dependency_same_name_different_spec_conflicts() {
        let a = dep_req("p1", "serde", "1.0");
        let b = dep_req("p2", "serde", "2.0");
        let (merged, conflicts) = CargoTomlRequirement::merge(&[&a, &b]);
        assert_eq!(
            conflicts.len(),
            1,
            "one per-entry conflict for the disagreeing dependency"
        );
        assert!(
            conflicts.iter().any(|c| c.key == "[dependencies].serde"),
            "the conflict names the dependency entry"
        );
        let scope = DependencyScope {
            kind: DependencyKind::Normal,
            target: None,
        };
        let kept = merged.dependencies.get(&scope).is_some_and(|m| {
            m.iter().all(|(_, assertion)| match assertion {
                DependencySetAssertion::Contains(map) => !map.contains_key("serde"),
                DependencySetAssertion::Excludes(_) | DependencySetAssertion::IsExactly(_) => true,
            })
        });
        assert!(kept, "the conflicting entry is dropped from the merged set");
    }

    #[test]
    fn package_lints_inherit_vs_inline_conflicts() {
        // The [lints] either/or key: one policy asserts the inherit opt-in,
        // another asserts inline tables -- cargo forbids the combination, so
        // the merge surfaces it as a conflict naming both policies.
        let a = CargoTomlRequirement {
            package_lints: Some(vec![(
                Provenance {
                    policy: "p-inherit".to_owned(),
                },
                PackageLintsAssertion::Inherit(true, "inherit the workspace tables".to_owned()),
            )]),
            ..CargoTomlRequirement::default()
        };
        let mut tools = BTreeMap::new();
        let _ = tools.insert(
            "clippy".to_owned(),
            LintLevelsAssertion::Contains(BTreeMap::from([(
                "unwrap_used".to_owned(),
                ("deny".to_owned(), None, "no unwraps".to_owned()),
            )])),
        );
        let b = CargoTomlRequirement {
            package_lints: Some(vec![(
                Provenance {
                    policy: "p-inline".to_owned(),
                },
                PackageLintsAssertion::Inline(tools),
            )]),
            ..CargoTomlRequirement::default()
        };
        let (merged, conflicts) = CargoTomlRequirement::merge(&[&a, &b]);
        assert!(
            merged.package_lints.is_none(),
            "the conflicted [lints] key is dropped, not written"
        );
        assert_eq!(conflicts.len(), 1, "exactly one conflict: {conflicts:?}");
        let entry = conflicts.first().expect("one conflict entry");
        assert_eq!(entry.key, "[lints]", "the conflict is at the [lints] key");
        let policies: Vec<&str> = entry
            .contributors
            .iter()
            .map(|(p, _)| p.policy.as_str())
            .collect();
        assert_eq!(
            policies,
            vec!["p-inherit", "p-inline"],
            "both policies are named"
        );
    }
}

#[cfg(test)]
mod f7_floor {
    use aqc_cargo_toml_engine::{CargoTomlRequirement, PackageFieldAssertion};
    use aqc_file_engine_core::Provenance;

    fn floor(policy: &str, v: &str) -> CargoTomlRequirement {
        let mut r = CargoTomlRequirement::default();
        let _ = r.package_fields.insert(
            "rust-version".to_owned(),
            vec![(
                Provenance {
                    policy: policy.to_owned(),
                },
                PackageFieldAssertion::AtLeastVersion(v.to_owned(), "floor".to_owned()),
            )],
        );
        r
    }

    #[test]
    fn two_floors_take_the_higher() {
        let a = floor("p1", "1.80");
        let b = floor("p2", "1.85");
        let (merged, conflicts) = CargoTomlRequirement::merge(&[&a, &b]);
        assert!(
            conflicts.is_empty(),
            "two floors compose, not conflict: {conflicts:?}"
        );
        let resolved = merged
            .package_fields
            .get("rust-version")
            .expect("the resolved rust-version floor must survive the merge");
        let version = match resolved.first().map(|(_, x)| x) {
            Some(PackageFieldAssertion::AtLeastVersion(v, _)) => Some(v.clone()),
            _ => None,
        };
        assert_eq!(version.as_deref(), Some("1.85"), "the higher floor wins");
    }
}
