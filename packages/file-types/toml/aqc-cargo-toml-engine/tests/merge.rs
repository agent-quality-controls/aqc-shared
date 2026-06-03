//! Behavior probes for `CargoTomlRequirement::merge` (the merge phase).
//!
//! The module is named `merge` so the test paths read `merge::disjoint`,
//! `merge::conflict`, `merge::identical` (the manifest's verification names).

use toml_edit as _;

mod merge {
    use aqc_cargo_toml_engine::{CargoTomlRequirement, PackageFieldAssertion};
    use aqc_file_engine_core::{MergedAssertion, Provenance};

    /// A requirement asserting one `[package].<field>` from one policy.
    fn field_req(
        policy: &str,
        field: &str,
        assertion: PackageFieldAssertion,
    ) -> CargoTomlRequirement {
        let mut r = CargoTomlRequirement::default();
        let _ = r.package_fields.insert(
            field.to_owned(),
            MergedAssertion {
                contributions: vec![(
                    Provenance {
                        policy: policy.to_owned(),
                    },
                    assertion,
                )],
            },
        );
        r
    }

    /// A requirement asserting `[package].edition == <edition>` from one policy.
    fn edition_req(policy: &str, edition: &str) -> CargoTomlRequirement {
        field_req(
            policy,
            "edition",
            PackageFieldAssertion::Equals(edition.to_owned()),
        )
    }

    #[test]
    fn disjoint() {
        let a = edition_req("p1", "2021");
        let b = field_req(
            "p2",
            "rust-version",
            PackageFieldAssertion::AtLeast("1.85".to_owned()),
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
        let a = edition_req("p1", "2021");
        let b = edition_req("p2", "2021");
        let (merged, conflicts) = CargoTomlRequirement::merge(&[&a, &b]);
        assert!(
            conflicts.is_empty(),
            "identical values agree (dedup for free)"
        );
        assert!(
            merged.package_fields.contains_key("edition"),
            "the agreed edition survives"
        );
    }

    #[test]
    fn conflict() {
        let a = edition_req("p1", "2021");
        let b = edition_req("p2", "2018");
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
}
