//! Behavior probes for `ClippyTomlRequirement::merge` (the merge phase).
//!
//! The module is named `merge` so the test paths read `merge::disjoint`,
//! `merge::conflict`, `merge::identical` (the manifest's verification names).

use toml_edit as _;

mod merge {
    use std::collections::BTreeMap;

    use aqc_clippy_toml_engine::{
        BanEntry, BansAssertion, BoolAssertion, ClippyTomlRequirement, ThresholdsAssertion,
    };
    use aqc_file_engine_core::{MergedAssertion, Provenance};

    /// Wrap one policy's assertion in a single-contribution `MergedAssertion`.
    fn one_contribution<A>(policy: &str, assertion: A) -> MergedAssertion<A> {
        MergedAssertion {
            contributions: vec![(
                Provenance {
                    policy: policy.to_owned(),
                },
                assertion,
            )],
        }
    }

    /// A requirement asserting `disallowed-methods` bans from one policy.
    fn bans_req(policy: &str, assertion: BansAssertion) -> ClippyTomlRequirement {
        ClippyTomlRequirement {
            disallowed_methods: Some(one_contribution(policy, assertion)),
            ..ClippyTomlRequirement::default()
        }
    }

    /// A requirement asserting `thresholds` from one policy.
    fn thresholds_req(policy: &str, assertion: ThresholdsAssertion) -> ClippyTomlRequirement {
        ClippyTomlRequirement {
            thresholds: Some(one_contribution(policy, assertion)),
            ..ClippyTomlRequirement::default()
        }
    }

    /// One `(name, (value, message))` map with a single entry.
    #[expect(
        clippy::type_complexity,
        reason = "BTreeMap<String, (value, message)> mirrors the value-carrying ThresholdsAssertion variants' shape."
    )]
    fn one_threshold(name: &str, value: u64, message: &str) -> BTreeMap<String, (u64, String)> {
        let mut m = BTreeMap::new();
        let _ = m.insert(name.to_owned(), (value, message.to_owned()));
        m
    }

    /// Count the ban entries in a requirement's `disallowed-methods` field.
    fn ban_count(req: &ClippyTomlRequirement) -> usize {
        req.disallowed_methods.as_ref().map_or(0, |m| {
            m.contributions
                .first()
                .map_or(0, |(_, assertion)| match assertion {
                    BansAssertion::Contains(v) | BansAssertion::IsExactly(v) => v.len(),
                    BansAssertion::Excludes(_) => 0,
                })
        })
    }

    /// A requirement asserting one boolean setting from one policy.
    fn bool_req(policy: &str, setting: &str, assertion: BoolAssertion) -> ClippyTomlRequirement {
        let mut r = ClippyTomlRequirement::default();
        let _ = r.bools.insert(
            setting.to_owned(),
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

    /// A requirement asserting `<setting> == <value>` (with a policy message).
    fn equals_req(
        policy: &str,
        setting: &str,
        value: bool,
        message: &str,
    ) -> ClippyTomlRequirement {
        bool_req(
            policy,
            setting,
            BoolAssertion::Equals(value, message.to_owned()),
        )
    }

    #[test]
    fn disjoint() {
        let a = equals_req("p1", "allow-dbg-in-tests", true, "tests may dbg");
        let b = equals_req("p2", "allow-print-in-tests", false, "no prints");
        let (merged, conflicts) = ClippyTomlRequirement::merge(&[&a, &b]);
        assert!(conflicts.is_empty(), "disjoint keys must not conflict");
        assert!(
            merged.bools.contains_key("allow-dbg-in-tests"),
            "first setting survives the merge"
        );
        assert!(
            merged.bools.contains_key("allow-print-in-tests"),
            "second setting survives the merge"
        );
    }

    #[test]
    fn identical() {
        // Same semantic value, different policy-authored messages: must agree.
        let a = equals_req("p1", "allow-dbg-in-tests", true, "reason from p1");
        let b = equals_req("p2", "allow-dbg-in-tests", true, "reason from p2");
        let (merged, conflicts) = ClippyTomlRequirement::merge(&[&a, &b]);
        assert!(
            conflicts.is_empty(),
            "same value with differing messages agrees (message is not the disagreement)"
        );
        assert!(
            merged.bools.contains_key("allow-dbg-in-tests"),
            "the agreed setting survives"
        );
    }

    #[test]
    fn conflict() {
        let a = equals_req("p1", "allow-dbg-in-tests", true, "tests may dbg");
        let b = equals_req("p2", "allow-dbg-in-tests", false, "tests may not dbg");
        let (merged, conflicts) = ClippyTomlRequirement::merge(&[&a, &b]);
        assert_eq!(
            conflicts.len(),
            1,
            "one per-key conflict for the disagreement"
        );
        assert!(
            conflicts.iter().any(|c| c.key == "allow-dbg-in-tests"),
            "the conflict names the disagreeing in-file key"
        );
        assert!(
            conflicts.iter().all(|c| c.contributors.len() == 2),
            "both disagreeing policies are named"
        );
        assert!(
            !merged.bools.contains_key("allow-dbg-in-tests"),
            "the conflicting setting is dropped, not written"
        );
    }

    #[test]
    fn bans_same_path_different_message_agrees() {
        let a = bans_req(
            "p1",
            BansAssertion::Contains(vec![BanEntry {
                path: "std::mem::forget".to_owned(),
                message: "leaks".to_owned(),
            }]),
        );
        let b = bans_req(
            "p2",
            BansAssertion::Contains(vec![BanEntry {
                path: "std::mem::forget".to_owned(),
                message: "use drop instead".to_owned(),
            }]),
        );
        let (merged, conflicts) = ClippyTomlRequirement::merge(&[&a, &b]);
        assert!(
            conflicts.is_empty(),
            "same ban path with differing messages agrees"
        );
        assert_eq!(
            ban_count(&merged),
            1,
            "the shared ban path is unioned to a single entry"
        );
    }

    #[test]
    fn bans_disjoint_paths_union() {
        let a = bans_req(
            "p1",
            BansAssertion::Contains(vec![BanEntry {
                path: "a::b".to_owned(),
                message: "m1".to_owned(),
            }]),
        );
        let b = bans_req(
            "p2",
            BansAssertion::Contains(vec![BanEntry {
                path: "c::d".to_owned(),
                message: "m2".to_owned(),
            }]),
        );
        let (merged, conflicts) = ClippyTomlRequirement::merge(&[&a, &b]);
        assert!(conflicts.is_empty(), "disjoint ban paths must not conflict");
        assert_eq!(
            ban_count(&merged),
            2,
            "both disjoint ban paths survive the union"
        );
    }

    #[test]
    fn thresholds_same_value_different_message_agrees() {
        let a = thresholds_req(
            "p1",
            ThresholdsAssertion::Equals(one_threshold("too-many-lines-threshold", 100, "m1")),
        );
        let b = thresholds_req(
            "p2",
            ThresholdsAssertion::Equals(one_threshold("too-many-lines-threshold", 100, "m2")),
        );
        let (_, conflicts) = ClippyTomlRequirement::merge(&[&a, &b]);
        assert!(
            conflicts.is_empty(),
            "same threshold value with differing messages agrees"
        );
    }

    #[test]
    fn thresholds_same_key_different_value_conflicts() {
        let a = thresholds_req(
            "p1",
            ThresholdsAssertion::Equals(one_threshold("too-many-lines-threshold", 100, "m1")),
        );
        let b = thresholds_req(
            "p2",
            ThresholdsAssertion::Equals(one_threshold("too-many-lines-threshold", 200, "m2")),
        );
        let (_, conflicts) = ClippyTomlRequirement::merge(&[&a, &b]);
        assert_eq!(
            conflicts.len(),
            1,
            "differing threshold values conflict on one key"
        );
        assert!(
            conflicts
                .iter()
                .any(|c| c.key == "[thresholds].too-many-lines-threshold"),
            "the conflict names the per-key in-file path"
        );
    }

    #[test]
    fn thresholds_cross_variant_conflicts() {
        let a = thresholds_req(
            "p1",
            ThresholdsAssertion::Equals(one_threshold("too-many-lines-threshold", 100, "m1")),
        );
        let b = thresholds_req(
            "p2",
            ThresholdsAssertion::AtMost(one_threshold("too-many-lines-threshold", 100, "m2")),
        );
        let (merged, conflicts) = ClippyTomlRequirement::merge(&[&a, &b]);
        assert_eq!(
            conflicts.len(),
            1,
            "mixed threshold variants fall through to a scalar conflict"
        );
        assert!(
            merged.thresholds.is_none(),
            "the conflicting thresholds field is dropped, not written"
        );
    }
}
