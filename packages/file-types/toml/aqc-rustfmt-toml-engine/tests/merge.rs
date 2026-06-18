use std::collections::BTreeMap;

use aqc_file_engine_core::{ConfigScalar, ForbiddenGlobRequirements, ListRequirements, Provenance};
use aqc_rustfmt_toml_engine::{
    RustfmtIgnorePathGlob, RustfmtListSetting, RustfmtScalarAssertion, RustfmtScalarSetting,
    RustfmtTomlRequirements,
};
use globset as _;
use toml_edit as _;

#[test]
fn merge_keeps_equal_scalar_requirements() {
    let (resolved, conflicts) = RustfmtTomlRequirements::merge(vec![
        (
            Provenance {
                policy: "left".to_owned(),
            },
            req(RustfmtScalarAssertion::Equals(
                ConfigScalar::Str("2024".to_owned()),
                "left".to_owned(),
            )),
        ),
        (
            Provenance {
                policy: "right".to_owned(),
            },
            req(RustfmtScalarAssertion::Equals(
                ConfigScalar::Str("2024".to_owned()),
                "right".to_owned(),
            )),
        ),
    ]);

    assert!(conflicts.is_empty(), "equal settings must merge cleanly");
    assert!(
        resolved
            .scalar_settings
            .contains_key(&RustfmtScalarSetting::Edition),
        "merged edition requirement must be retained"
    );
}

#[test]
fn merge_reports_conflicting_scalar_requirements() {
    let (_resolved, conflicts) = RustfmtTomlRequirements::merge(vec![
        (
            Provenance {
                policy: "left".to_owned(),
            },
            req(RustfmtScalarAssertion::Equals(
                ConfigScalar::Str("2021".to_owned()),
                "left".to_owned(),
            )),
        ),
        (
            Provenance {
                policy: "right".to_owned(),
            },
            req(RustfmtScalarAssertion::Equals(
                ConfigScalar::Str("2024".to_owned()),
                "right".to_owned(),
            )),
        ),
    ]);

    assert_eq!(
        conflicts.len(),
        1,
        "conflicting edition must produce one conflict"
    );
    assert_eq!(conflicts[0].key, "edition", "conflict key must be file key");
}

#[test]
fn forbidden_ignore_path_globs_dedupe_attribution() {
    let (resolved, conflicts) = RustfmtTomlRequirements::merge(vec![
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
    ]);

    assert!(conflicts.is_empty(), "same glob should merge cleanly");
    assert_eq!(
        resolved.forbidden_ignore_path_globs.globs["target/**"]
            .collected
            .len(),
        2,
        "same glob should retain both contributors"
    );
}

#[test]
fn forbidden_ignore_path_glob_conflicts_with_required_ignore_path() {
    let (resolved, conflicts) = RustfmtTomlRequirements::merge(vec![
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
    ]);

    assert!(
        conflicts
            .iter()
            .any(|conflict| conflict.reason == "ignore-path-glob-forbids-required-path"),
        "required ignore entry matching glob should conflict"
    );
    assert!(
        resolved
            .ignore_glob_conflicts
            .path_globs
            .contains("target/**"),
        "conflicting glob should be blocked during reconcile"
    );
}

fn req(assertion: RustfmtScalarAssertion) -> RustfmtTomlRequirements {
    RustfmtTomlRequirements {
        scalar_settings: BTreeMap::from([(RustfmtScalarSetting::Edition, assertion)]),
        ..RustfmtTomlRequirements::default()
    }
}

fn ignore_globs(globs: Vec<(&str, &str)>) -> RustfmtTomlRequirements {
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
