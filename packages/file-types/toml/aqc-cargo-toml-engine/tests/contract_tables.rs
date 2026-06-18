#![expect(
    clippy::as_conversions,
    clippy::field_reassign_with_default,
    clippy::missing_const_for_fn,
    clippy::type_complexity,
    reason = "Contract tests keep compact fixture construction close to each assertion."
)]

use std::collections::{BTreeMap, BTreeSet};

use aqc_cargo_toml_engine as cargo;
use aqc_file_engine_core as engine_core;
use engine_core::Engine;
use globset as _;
use toml_edit as _;

fn prov() -> engine_core::Provenance {
    engine_core::Provenance {
        policy: "contract".to_owned(),
    }
}

fn output(req: cargo::CargoTomlRequirements, current: Option<&[u8]>) -> engine_core::EngineOutput {
    let reqs = vec![(
        prov(),
        Box::new(req) as Box<dyn engine_core::EngineRequirement>,
    )];
    cargo::CargoTomlEngine.reconcile(current, &reqs)
}

fn normal_scope() -> cargo::DependencyScope {
    cargo::DependencyScope {
        kind: cargo::DependencyKind::Normal,
        target: None,
    }
}

fn dep_items(
    required: BTreeMap<String, (cargo::DependencySpec, String)>,
    banned: BTreeMap<String, String>,
    closed: Option<String>,
) -> engine_core::ItemRequirements<cargo::DependencyRequirement> {
    engine_core::ItemRequirements {
        required: required
            .into_iter()
            .map(|(file_key, (value, msg))| {
                (
                    cargo::DependencyRequirement {
                        file_key: Some(file_key),
                        value,
                    },
                    msg,
                )
            })
            .collect(),
        banned: banned
            .into_iter()
            .map(|(file_key, msg)| {
                (
                    cargo::DependencyRequirement {
                        file_key: Some(file_key),
                        value: cargo::DependencySpec::default(),
                    },
                    msg,
                )
            })
            .collect(),
        closed,
    }
}

fn keyed_items<Entry: Default>(
    required: BTreeMap<String, (Entry, String)>,
    banned: BTreeMap<String, String>,
    closed: Option<String>,
) -> engine_core::ItemRequirements<engine_core::KeyedItem<Entry>> {
    engine_core::ItemRequirements {
        required: required
            .into_iter()
            .map(|(file_key, (value, msg))| (engine_core::KeyedItem { file_key, value }, msg))
            .collect(),
        banned: banned
            .into_iter()
            .map(|(file_key, msg)| {
                (
                    engine_core::KeyedItem {
                        file_key,
                        value: Entry::default(),
                    },
                    msg,
                )
            })
            .collect(),
        closed,
    }
}

#[test]
fn dependency_table_required_entry_writes_version() {
    let table = dep_items(
        BTreeMap::from([(
            "serde".to_owned(),
            (
                cargo::DependencySpec {
                    version: Some("1".to_owned()),
                    ..cargo::DependencySpec::default()
                },
                "serde".to_owned(),
            ),
        )]),
        BTreeMap::new(),
        None,
    );
    let mut req = cargo::CargoTomlRequirements::default();
    let _ = req.dependencies.insert(normal_scope(), table);
    let out = output(req, None);
    let text =
        String::from_utf8(out.expected_bytes).expect("engine output should be valid UTF-8 TOML");
    assert!(text.contains("[dependencies]"));
    assert!(text.contains("serde = \"1\""));
}

#[test]
fn dependency_table_banned_entry_removes_key() {
    let table = dep_items(
        BTreeMap::new(),
        BTreeMap::from([("serde".to_owned(), "no serde".to_owned())]),
        None,
    );
    let mut req = cargo::CargoTomlRequirements::default();
    let _ = req.dependencies.insert(normal_scope(), table);
    let out = output(
        req,
        Some(b"[dependencies]\nserde = \"1\"\ntoml = \"0.8\"\n"),
    );
    let text =
        String::from_utf8(out.expected_bytes).expect("engine output should be valid UTF-8 TOML");
    assert!(!text.contains("serde = "));
    assert!(text.contains("toml = \"0.8\""));
}

#[test]
fn workspace_dependencies_use_same_table_product() {
    let table = dep_items(
        BTreeMap::from([(
            "serde".to_owned(),
            (
                cargo::DependencySpec {
                    version: Some("1".to_owned()),
                    ..cargo::DependencySpec::default()
                },
                "serde".to_owned(),
            ),
        )]),
        BTreeMap::new(),
        None,
    );
    let mut req = cargo::CargoTomlRequirements::default();
    req.workspace_dependencies = Some(table);
    let out = output(req, None);
    let text =
        String::from_utf8(out.expected_bytes).expect("engine output should be valid UTF-8 TOML");
    assert!(text.contains("[workspace.dependencies]"));
    assert!(text.contains("serde = \"1\""));
}

#[test]
fn features_table_writes_required_feature_entry() {
    let mut enabled = BTreeSet::new();
    let _ = enabled.insert("dep:serde".to_owned());
    let entry = cargo::FeatureMembers { members: enabled };
    let mut req = cargo::CargoTomlRequirements::default();
    req.features = Some(keyed_items(
        BTreeMap::from([("default".to_owned(), (entry, "default".to_owned()))]),
        BTreeMap::new(),
        None,
    ));
    let out = output(req, None);
    let text =
        String::from_utf8(out.expected_bytes).expect("engine output should be valid UTF-8 TOML");
    assert!(text.contains("[features]"));
    assert!(text.contains("default = [\"dep:serde\"]"));
}

#[test]
fn required_empty_feature_writes_empty_array_when_absent() {
    let req = cargo::CargoTomlRequirements {
        features: Some(keyed_items(
            BTreeMap::from([(
                "empty".to_owned(),
                (
                    cargo::FeatureMembers {
                        members: BTreeSet::new(),
                    },
                    "empty".to_owned(),
                ),
            )]),
            BTreeMap::new(),
            None,
        )),
        ..cargo::CargoTomlRequirements::default()
    };
    let out = output(req, None);
    let text =
        String::from_utf8(out.expected_bytes).expect("engine output should be valid UTF-8 TOML");
    assert!(text.contains("empty = []"));
}

#[test]
fn required_empty_feature_overwrites_malformed_value() {
    let req = cargo::CargoTomlRequirements {
        features: Some(keyed_items(
            BTreeMap::from([(
                "empty".to_owned(),
                (
                    cargo::FeatureMembers {
                        members: BTreeSet::new(),
                    },
                    "empty".to_owned(),
                ),
            )]),
            BTreeMap::new(),
            None,
        )),
        ..cargo::CargoTomlRequirements::default()
    };
    let out = output(req, Some(b"[features]\nempty = \"bad\"\n"));
    let text =
        String::from_utf8(out.expected_bytes).expect("engine output should be valid UTF-8 TOML");
    assert!(text.contains("empty = []"));
    assert!(!out.findings.is_empty());
}

#[test]
fn profile_nested_fields_are_addressable() {
    let mut profile = cargo::ProfileRequirements::default();
    let _ = profile.fields.insert(
        "opt-level".to_owned(),
        cargo::ProfileFieldAssertion::Equals(engine_core::ConfigScalar::Int(3), "opt".to_owned()),
    );
    let mut req = cargo::CargoTomlRequirements::default();
    let _ = req.profiles.insert("release".to_owned(), profile);
    let out = output(req, None);
    let text =
        String::from_utf8(out.expected_bytes).expect("engine output should be valid UTF-8 TOML");
    assert!(text.contains("[profile.release]"));
    assert!(text.contains("opt-level = 3"));
}

#[test]
fn named_target_fields_are_addressable() {
    let mut targets = cargo::TargetRequirements::default();
    let _ = targets.bin_targets.insert(
        "cli".to_owned(),
        cargo::TargetTableAssertion::Fields(BTreeMap::from([(
            "path".to_owned(),
            cargo::TargetFieldAssertion::Equals(
                engine_core::ConfigScalar::Str("src/bin/cli.rs".to_owned()),
                "path".to_owned(),
            ),
        )])),
    );
    let mut req = cargo::CargoTomlRequirements::default();
    req.targets = targets;
    let out = output(req, None);
    let text =
        String::from_utf8(out.expected_bytes).expect("engine output should be valid UTF-8 TOML");
    assert!(text.contains("[[bin]]"));
    assert!(text.contains("name = \"cli\""));
    assert!(text.contains("path = \"src/bin/cli.rs\""));
}

#[test]
fn target_lib_list_field_uses_unified_list_requirements() {
    let mut targets = cargo::TargetRequirements::default();
    let _ = targets.lib_fields.insert(
        "required-features".to_owned(),
        cargo::TargetFieldAssertion::List(engine_core::ListRequirements {
            contains: BTreeMap::from([("feat-a".to_owned(), "feature".to_owned())]),
            excludes: BTreeMap::new(),
            exact: None,
        }),
    );
    let mut req = cargo::CargoTomlRequirements::default();
    req.targets = targets;
    let out = output(req, None);
    let text =
        String::from_utf8(out.expected_bytes).expect("engine output should be valid UTF-8 TOML");
    assert!(text.contains("[lib]"));
    assert!(text.contains("required-features = [\"feat-a\"]"));
}
