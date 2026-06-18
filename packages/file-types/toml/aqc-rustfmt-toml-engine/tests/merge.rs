use std::collections::BTreeMap;

use aqc_file_engine_core::{ConfigScalar, Provenance};
use aqc_rustfmt_toml_engine::{
    RustfmtScalarAssertion, RustfmtScalarSetting, RustfmtTomlRequirements,
};
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

fn req(assertion: RustfmtScalarAssertion) -> RustfmtTomlRequirements {
    RustfmtTomlRequirements {
        scalar_settings: BTreeMap::from([(RustfmtScalarSetting::Edition, assertion)]),
        ..RustfmtTomlRequirements::default()
    }
}
