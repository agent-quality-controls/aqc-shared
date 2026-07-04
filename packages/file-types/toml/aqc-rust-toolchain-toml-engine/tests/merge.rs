use std::collections::BTreeMap;

use aqc_file_engine_core::{ConfigScalar, ListRequirements, Provenance, ScalarAssertion};
use aqc_rust_toolchain_toml_engine::{
    RustToolchainListSetting, RustToolchainScalarSetting, RustToolchainTomlRequirements,
};
use aqc_toml_engine_core as _;
use toml_edit as _;

#[test]
fn merge_keeps_equal_scalar_requirements() {
    let (resolved, conflicts) = RustToolchainTomlRequirements::merge(vec![
        (
            Provenance {
                policy: "left".to_owned(),
            },
            req(ScalarAssertion::Equals(
                ConfigScalar::Str("stable".to_owned()),
                "left".to_owned(),
            )),
        ),
        (
            Provenance {
                policy: "right".to_owned(),
            },
            req(ScalarAssertion::Equals(
                ConfigScalar::Str("stable".to_owned()),
                "right".to_owned(),
            )),
        ),
    ]);

    assert!(conflicts.is_empty(), "equal settings must merge cleanly");
    assert!(
        resolved
            .scalar_settings
            .contains_key(&RustToolchainScalarSetting::Channel),
        "merged channel requirement must be retained"
    );
}

#[test]
fn merge_reports_conflicting_scalar_requirements() {
    let (_resolved, conflicts) = RustToolchainTomlRequirements::merge(vec![
        (
            Provenance {
                policy: "left".to_owned(),
            },
            req(ScalarAssertion::Equals(
                ConfigScalar::Str("stable".to_owned()),
                "left".to_owned(),
            )),
        ),
        (
            Provenance {
                policy: "right".to_owned(),
            },
            req(ScalarAssertion::Equals(
                ConfigScalar::Str("nightly".to_owned()),
                "right".to_owned(),
            )),
        ),
    ]);

    assert_eq!(
        conflicts.len(),
        1,
        "conflicting channel must produce one conflict"
    );
    assert_eq!(
        conflicts.first().map(|conflict| conflict.key.as_str()),
        Some("channel"),
        "conflict key must be file key"
    );
}

#[test]
fn exact_lists_are_unordered_at_merge() {
    let (resolved, conflicts) = RustToolchainTomlRequirements::merge(vec![
        (
            Provenance {
                policy: "left".to_owned(),
            },
            list_req(vec!["rustfmt", "clippy"]),
        ),
        (
            Provenance {
                policy: "right".to_owned(),
            },
            list_req(vec!["clippy", "rustfmt"]),
        ),
    ]);

    assert!(
        conflicts.is_empty(),
        "same component set in different order must merge cleanly"
    );
    let exact = resolved
        .list_settings
        .get(&RustToolchainListSetting::Components)
        .and_then(|list| list.exact.as_ref())
        .expect("exact components should resolve");
    assert_eq!(
        exact.merged,
        ["clippy".to_owned(), "rustfmt".to_owned()],
        "exact components should be deterministic"
    );
}

#[test]
fn rejects_scalar_operations_outside_setting_type() {
    let (resolved, conflicts) = RustToolchainTomlRequirements::merge(vec![(
        Provenance {
            policy: "policy".to_owned(),
        },
        RustToolchainTomlRequirements {
            scalar_settings: BTreeMap::from([(
                RustToolchainScalarSetting::Channel,
                ScalarAssertion::Equals(ConfigScalar::Bool(true), "wrong".to_owned()),
            )]),
            ..RustToolchainTomlRequirements::default()
        },
    )]);

    assert!(resolved.scalar_settings.is_empty());
    assert_eq!(
        conflicts
            .iter()
            .filter(|conflict| conflict.reason == "scalar-operation-unsupported")
            .count(),
        1
    );
}

fn req(assertion: ScalarAssertion<ConfigScalar>) -> RustToolchainTomlRequirements {
    RustToolchainTomlRequirements {
        scalar_settings: BTreeMap::from([(RustToolchainScalarSetting::Channel, assertion)]),
        ..RustToolchainTomlRequirements::default()
    }
}

fn list_req(values: Vec<&str>) -> RustToolchainTomlRequirements {
    RustToolchainTomlRequirements {
        list_settings: BTreeMap::from([(
            RustToolchainListSetting::Components,
            ListRequirements {
                exact: Some((
                    values.into_iter().map(ToOwned::to_owned).collect(),
                    "components".to_owned(),
                )),
                ..ListRequirements::default()
            },
        )]),
        ..RustToolchainTomlRequirements::default()
    }
}
