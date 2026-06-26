#![expect(
    clippy::indexing_slicing,
    reason = "These assertions index known merged fixture keys."
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
fn dependency_partial_specs_compose_fieldwise() {
    let mut left = BTreeMap::new();
    let _ = left.insert(
        "serde".to_owned(),
        (
            cargo::DependencySpec {
                version: Some("1".to_owned()),
                ..cargo::DependencySpec::default()
            },
            "v".to_owned(),
        ),
    );
    let mut features = BTreeSet::new();
    let _ = features.insert("derive".to_owned());
    let mut right = BTreeMap::new();
    let _ = right.insert(
        "serde".to_owned(),
        (
            cargo::DependencySpec {
                features,
                ..cargo::DependencySpec::default()
            },
            "f".to_owned(),
        ),
    );
    let (merged, conflicts) = cargo::CargoTomlRequirements::merge(vec![
        (
            prov("p1"),
            dep_req(KeyedFixture {
                required: left,
                forbidden: BTreeMap::new(),
                closed: None,
            }),
        ),
        (
            prov("p2"),
            dep_req(KeyedFixture {
                required: right,
                forbidden: BTreeMap::new(),
                closed: None,
            }),
        ),
    ]);
    let spec: &cargo::DependencySpec = &merged.dependencies[&normal_scope()].required
        [&cargo::DependencyIdentity::LocalKey("serde".to_owned())]
        .merged
        .value;
    assert!(conflicts.is_empty());
    assert_eq!(spec.version.as_deref(), Some("1"));
    assert!(spec.features.contains("derive"));
}

#[test]
fn dependency_incompatible_same_field_specs_conflict() {
    let mut left = BTreeMap::new();
    let _ = left.insert("serde".to_owned(), (dep_spec(Some("1")), "one".to_owned()));
    let mut right = BTreeMap::new();
    let _ = right.insert("serde".to_owned(), (dep_spec(Some("2")), "two".to_owned()));
    let findings = cargo_findings(vec![
        (
            prov("p1"),
            dep_req(KeyedFixture {
                required: left,
                forbidden: BTreeMap::new(),
                closed: None,
            }),
        ),
        (
            prov("p2"),
            dep_req(KeyedFixture {
                required: right,
                forbidden: BTreeMap::new(),
                closed: None,
            }),
        ),
    ]);
    assert!(matches!(
        findings
            .iter()
            .find(|f| matches!(f, engine_core::Finding::ConflictingRequirements { .. })),
        Some(engine_core::Finding::ConflictingRequirements { .. })
    ));
}

#[test]
fn dependency_each_field_composes_independently() {
    let mut required = BTreeMap::new();
    let _ = required.insert(
        "serde".to_owned(),
        (
            cargo::DependencySpec {
                default_features: Some(false),
                optional: Some(true),
                registry: Some("crates-io".to_owned()),
                package: Some("serde-renamed".to_owned()),
                ..cargo::DependencySpec::default()
            },
            "attrs".to_owned(),
        ),
    );
    let (merged, conflicts) = cargo::CargoTomlRequirements::merge(vec![(
        prov("p1"),
        dep_req(KeyedFixture {
            required,
            forbidden: BTreeMap::new(),
            closed: None,
        }),
    )]);
    let spec: &cargo::DependencySpec = &merged.dependencies[&normal_scope()].required
        [&cargo::DependencyIdentity::Package("serde-renamed".to_owned())]
        .merged
        .value;
    assert!(conflicts.is_empty());
    assert_eq!(spec.default_features, Some(false));
    assert_eq!(spec.optional, Some(true));
    assert_eq!(spec.registry.as_deref(), Some("crates-io"));
    assert_eq!(spec.package.as_deref(), Some("serde-renamed"));
}
