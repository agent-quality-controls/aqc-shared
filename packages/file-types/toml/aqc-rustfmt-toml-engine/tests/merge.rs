#![allow(
    clippy::expect_used,
    reason = "Tests use expect to fail loudly when fixture invariants are broken."
)]
use aqc_toml_engine_core as _;
use std::collections::BTreeMap;

use aqc_file_engine_core::{
    ConfigScalar, ForbiddenGlobRequirements, ListRequirements, Provenance, ScalarAssertion,
};
use aqc_rustfmt_toml_engine::{
    RustfmtIgnorePathGlob, RustfmtListSetting, RustfmtScalarSetting, RustfmtTomlRequirements,
};
use globset as _;
use toml_edit as _;

type IgnoreGlobCases<'a> = Vec<(&'a str, &'a str)>;

#[test]
fn merge_keeps_equal_scalar_requirements() {
    let resolved = RustfmtTomlRequirements::merge(vec![
        (
            Provenance {
                policy: "left".to_owned(),
            },
            req(ScalarAssertion::Equals(
                ConfigScalar::Str("2024".to_owned()),
                "left".to_owned(),
            )),
        ),
        (
            Provenance {
                policy: "right".to_owned(),
            },
            req(ScalarAssertion::Equals(
                ConfigScalar::Str("2024".to_owned()),
                "right".to_owned(),
            )),
        ),
    ])
    .expect("equal settings must merge cleanly");

    assert!(
        resolved
            .scalar_settings()
            .contains_key(&RustfmtScalarSetting::Edition),
        "merged edition requirement must be retained"
    );
}

#[test]
fn merge_reports_conflicting_scalar_requirements() {
    let conflicts = RustfmtTomlRequirements::merge(vec![
        (
            Provenance {
                policy: "left".to_owned(),
            },
            req(ScalarAssertion::Equals(
                ConfigScalar::Str("2021".to_owned()),
                "left".to_owned(),
            )),
        ),
        (
            Provenance {
                policy: "right".to_owned(),
            },
            req(ScalarAssertion::Equals(
                ConfigScalar::Str("2024".to_owned()),
                "right".to_owned(),
            )),
        ),
    ])
    .expect_err("conflicting edition must not expose a resolved root");

    assert_eq!(
        conflicts.len(),
        1,
        "conflicting edition must produce one conflict"
    );
    let conflict = conflicts
        .first()
        .expect("one scalar conflict should be present");
    assert_eq!(conflict.key, "edition", "conflict key must be file key");
}

#[test]
fn rustfmt_requirements_use_core_scalar_assertions() {
    let resolved = RustfmtTomlRequirements::merge(vec![(
        Provenance {
            policy: "policy".to_owned(),
        },
        req(ScalarAssertion::Equals(
            ConfigScalar::Str("2024".to_owned()),
            "edition".to_owned(),
        )),
    )])
    .expect("supported scalar assertion must merge");

    assert!(
        resolved
            .scalar_settings()
            .contains_key(&RustfmtScalarSetting::Edition)
    );
}

#[test]
fn rustfmt_rejects_scalar_operations_outside_setting_type() {
    let conflicts = RustfmtTomlRequirements::merge(vec![(
        Provenance {
            policy: "policy".to_owned(),
        },
        RustfmtTomlRequirements {
            scalar_settings: BTreeMap::from([
                (
                    RustfmtScalarSetting::HardTabs,
                    ScalarAssertion::OneOf(
                        std::collections::BTreeSet::from([ConfigScalar::Bool(true)]),
                        "bool oneof".to_owned(),
                    ),
                ),
                (
                    RustfmtScalarSetting::MaxWidth,
                    ScalarAssertion::Equals(
                        ConfigScalar::Str("100".to_owned()),
                        "wrong type".to_owned(),
                    ),
                ),
                (
                    RustfmtScalarSetting::Edition,
                    ScalarAssertion::AtLeast(
                        ConfigScalar::Str("2021".to_owned()),
                        "ordered".to_owned(),
                    ),
                ),
            ]),
            ..RustfmtTomlRequirements::default()
        },
    )])
    .expect_err("unsupported scalar operations must not expose a resolved root");

    assert_eq!(
        conflicts
            .iter()
            .filter(|conflict| conflict.reason == "scalar-operation-unsupported")
            .count(),
        3
    );
}

#[test]
#[allow(
    clippy::too_many_lines,
    reason = "Exhaustive rustfmt scalar-kind test covers bool, int, text, and unsupported operations."
)]
fn rustfmt_scalar_setting_kind_validation_covers_all_kinds() {
    let valid = RustfmtTomlRequirements::merge(vec![(
        Provenance {
            policy: "valid".to_owned(),
        },
        RustfmtTomlRequirements {
            scalar_settings: BTreeMap::from([
                (
                    RustfmtScalarSetting::UseTryShorthand,
                    ScalarAssertion::Present("bool present".to_owned()),
                ),
                (
                    RustfmtScalarSetting::FnCallWidth,
                    ScalarAssertion::Absent("int absent".to_owned()),
                ),
                (
                    RustfmtScalarSetting::Version,
                    ScalarAssertion::Present("text present".to_owned()),
                ),
            ]),
            ..RustfmtTomlRequirements::default()
        },
    )])
    .expect("supported scalar operations must merge");
    assert!(
        valid
            .scalar_settings()
            .contains_key(&RustfmtScalarSetting::UseTryShorthand)
    );
    assert!(
        valid
            .scalar_settings()
            .contains_key(&RustfmtScalarSetting::FnCallWidth)
    );
    assert!(
        valid
            .scalar_settings()
            .contains_key(&RustfmtScalarSetting::Version)
    );

    let conflicts = RustfmtTomlRequirements::merge(vec![(
        Provenance {
            policy: "policy".to_owned(),
        },
        RustfmtTomlRequirements {
            scalar_settings: BTreeMap::from([
                (
                    RustfmtScalarSetting::HardTabs,
                    ScalarAssertion::Equals(
                        ConfigScalar::Str("true".to_owned()),
                        "wrong bool".to_owned(),
                    ),
                ),
                (
                    RustfmtScalarSetting::MaxWidth,
                    ScalarAssertion::OneOf(
                        std::collections::BTreeSet::from([ConfigScalar::Str("100".to_owned())]),
                        "wrong int oneof".to_owned(),
                    ),
                ),
                (
                    RustfmtScalarSetting::Edition,
                    ScalarAssertion::Equals(ConfigScalar::Int(2024), "wrong text".to_owned()),
                ),
                (
                    RustfmtScalarSetting::StyleEdition,
                    ScalarAssertion::OneOf(
                        std::collections::BTreeSet::from([ConfigScalar::Int(2024)]),
                        "wrong text oneof".to_owned(),
                    ),
                ),
                (
                    RustfmtScalarSetting::TabSpaces,
                    ScalarAssertion::Range(
                        ConfigScalar::Int(2),
                        ConfigScalar::Int(4),
                        "range".to_owned(),
                    ),
                ),
            ]),
            ..RustfmtTomlRequirements::default()
        },
    )])
    .expect_err("invalid scalar kinds must not expose a resolved root");
    assert_eq!(
        conflicts
            .iter()
            .filter(|conflict| conflict.reason == "scalar-operation-unsupported")
            .count(),
        5
    );
}

#[test]
fn forbidden_ignore_path_globs_dedupe_attribution() {
    let resolved = RustfmtTomlRequirements::merge(vec![
        (
            Provenance {
                policy: "left".to_owned(),
            },
            ignore_globs(vec![("target/**", "left")]),
        ),
        (
            Provenance {
                policy: "right".to_owned(),
            },
            ignore_globs(vec![("target/**", "right")]),
        ),
    ])
    .expect("same glob should merge cleanly");

    assert_eq!(
        resolved
            .forbidden_ignore_path_globs()
            .globs
            .get("target/**")
            .expect("merged forbidden glob should exist")
            .collected
            .len(),
        2,
        "same glob should retain both contributors"
    );
}

#[test]
fn forbidden_ignore_path_glob_conflicts_with_required_ignore_path() {
    let conflicts = RustfmtTomlRequirements::merge(vec![
        (
            Provenance {
                policy: "required".to_owned(),
            },
            RustfmtTomlRequirements {
                list_settings: BTreeMap::from([(
                    RustfmtListSetting::Ignore,
                    ListRequirements {
                        contains: BTreeMap::from([(
                            "target/generated".to_owned(),
                            "must ignore generated".to_owned(),
                        )]),
                        ..ListRequirements::default()
                    },
                )]),
                ..RustfmtTomlRequirements::default()
            },
        ),
        (
            Provenance {
                policy: "forbid".to_owned(),
            },
            ignore_globs(vec![("target/**", "do not ignore target")]),
        ),
    ])
    .expect_err("required path matched by a forbidden glob must fail resolution");

    let conflict = conflicts
        .iter()
        .find(|conflict| conflict.reason == "ignore-path-glob-forbids-required-path")
        .expect("required ignore entry matching glob should conflict");
    assert_eq!(conflict.key, "ignore.target/generated");
    assert_eq!(
        conflict.contributors,
        vec![
            (
                Provenance {
                    policy: "required".to_owned(),
                },
                "required".to_owned(),
            ),
            (
                Provenance {
                    policy: "forbid".to_owned(),
                },
                "forbidden".to_owned(),
            ),
        ]
    );
}

fn req(assertion: ScalarAssertion<ConfigScalar>) -> RustfmtTomlRequirements {
    RustfmtTomlRequirements {
        scalar_settings: BTreeMap::from([(RustfmtScalarSetting::Edition, assertion)]),
        ..RustfmtTomlRequirements::default()
    }
}

fn ignore_globs(globs: IgnoreGlobCases<'_>) -> RustfmtTomlRequirements {
    RustfmtTomlRequirements {
        forbidden_ignore_path_globs: ForbiddenGlobRequirements {
            globs: globs
                .into_iter()
                .map(|(glob, msg)| {
                    (
                        RustfmtIgnorePathGlob {
                            glob: glob.to_owned(),
                        },
                        msg.to_owned(),
                    )
                })
                .collect(),
        },
        ..RustfmtTomlRequirements::default()
    }
}
