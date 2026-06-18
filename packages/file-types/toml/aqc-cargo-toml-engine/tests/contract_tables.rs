use std::collections::{BTreeMap, BTreeSet};

use aqc_cargo_toml_engine::{
    CargoTomlEngine, CargoTomlRequirements, DependencyKind, DependencyRequirement, DependencyScope,
    DependencySpec, FeatureMembers, ProfileFieldAssertion, ProfileRequirements,
    TargetFieldAssertion, TargetRequirements, TargetTableAssertion,
};
use aqc_file_engine_core::{
    ConfigScalar, Engine, EngineOutput, EngineRequirement, ItemRequirements, KeyedItem,
    ListRequirements, Provenance,
};
use globset as _;
use toml_edit as _;

fn prov() -> Provenance {
    Provenance {
        policy: "contract".to_owned(),
    }
}

fn output(req: CargoTomlRequirements, current: Option<&[u8]>) -> EngineOutput {
    let reqs = vec![(prov(), Box::new(req) as Box<dyn EngineRequirement>)];
    CargoTomlEngine.reconcile(current, &reqs)
}

fn normal_scope() -> DependencyScope {
    DependencyScope {
        kind: DependencyKind::Normal,
        target: None,
    }
}

fn dep_items(
    required: BTreeMap<String, (DependencySpec, String)>,
    banned: BTreeMap<String, String>,
    closed: Option<String>,
) -> ItemRequirements<DependencyRequirement> {
    ItemRequirements {
        required: required
            .into_iter()
            .map(|(file_key, (value, msg))| {
                (
                    DependencyRequirement {
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
                    DependencyRequirement {
                        file_key: Some(file_key),
                        value: DependencySpec::default(),
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
) -> ItemRequirements<KeyedItem<Entry>> {
    ItemRequirements {
        required: required
            .into_iter()
            .map(|(file_key, (value, msg))| (KeyedItem { file_key, value }, msg))
            .collect(),
        banned: banned
            .into_iter()
            .map(|(file_key, msg)| {
                (
                    KeyedItem {
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
                DependencySpec {
                    version: Some("1".to_owned()),
                    ..DependencySpec::default()
                },
                "serde".to_owned(),
            ),
        )]),
        BTreeMap::new(),
        None,
    );
    let mut req = CargoTomlRequirements::default();
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
    let mut req = CargoTomlRequirements::default();
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
                DependencySpec {
                    version: Some("1".to_owned()),
                    ..DependencySpec::default()
                },
                "serde".to_owned(),
            ),
        )]),
        BTreeMap::new(),
        None,
    );
    let mut req = CargoTomlRequirements::default();
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
    let entry = FeatureMembers { members: enabled };
    let mut req = CargoTomlRequirements::default();
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
    let req = CargoTomlRequirements {
        features: Some(keyed_items(
            BTreeMap::from([(
                "empty".to_owned(),
                (
                    FeatureMembers {
                        members: BTreeSet::new(),
                    },
                    "empty".to_owned(),
                ),
            )]),
            BTreeMap::new(),
            None,
        )),
        ..CargoTomlRequirements::default()
    };
    let out = output(req, None);
    let text =
        String::from_utf8(out.expected_bytes).expect("engine output should be valid UTF-8 TOML");
    assert!(text.contains("empty = []"));
}

#[test]
fn required_empty_feature_overwrites_malformed_value() {
    let req = CargoTomlRequirements {
        features: Some(keyed_items(
            BTreeMap::from([(
                "empty".to_owned(),
                (
                    FeatureMembers {
                        members: BTreeSet::new(),
                    },
                    "empty".to_owned(),
                ),
            )]),
            BTreeMap::new(),
            None,
        )),
        ..CargoTomlRequirements::default()
    };
    let out = output(req, Some(b"[features]\nempty = \"bad\"\n"));
    let text =
        String::from_utf8(out.expected_bytes).expect("engine output should be valid UTF-8 TOML");
    assert!(text.contains("empty = []"));
    assert!(!out.findings.is_empty());
}

#[test]
fn profile_nested_fields_are_addressable() {
    let mut profile = ProfileRequirements::default();
    let _ = profile.fields.insert(
        "opt-level".to_owned(),
        ProfileFieldAssertion::Equals(ConfigScalar::Int(3), "opt".to_owned()),
    );
    let mut req = CargoTomlRequirements::default();
    let _ = req.profiles.insert("release".to_owned(), profile);
    let out = output(req, None);
    let text =
        String::from_utf8(out.expected_bytes).expect("engine output should be valid UTF-8 TOML");
    assert!(text.contains("[profile.release]"));
    assert!(text.contains("opt-level = 3"));
}

#[test]
fn named_target_fields_are_addressable() {
    let mut targets = TargetRequirements::default();
    let _ = targets.bin_targets.insert(
        "cli".to_owned(),
        TargetTableAssertion::Fields(BTreeMap::from([(
            "path".to_owned(),
            TargetFieldAssertion::Equals(
                ConfigScalar::Str("src/bin/cli.rs".to_owned()),
                "path".to_owned(),
            ),
        )])),
    );
    let mut req = CargoTomlRequirements::default();
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
    let mut targets = TargetRequirements::default();
    let _ = targets.lib_fields.insert(
        "required-features".to_owned(),
        TargetFieldAssertion::List(ListRequirements {
            contains: BTreeMap::from([("feat-a".to_owned(), "feature".to_owned())]),
            excludes: BTreeMap::new(),
            exact: None,
        }),
    );
    let mut req = CargoTomlRequirements::default();
    req.targets = targets;
    let out = output(req, None);
    let text =
        String::from_utf8(out.expected_bytes).expect("engine output should be valid UTF-8 TOML");
    assert!(text.contains("[lib]"));
    assert!(text.contains("required-features = [\"feat-a\"]"));
}
