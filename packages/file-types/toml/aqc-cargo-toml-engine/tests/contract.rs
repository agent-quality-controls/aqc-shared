#![expect(
    clippy::as_conversions,
    clippy::field_reassign_with_default,
    clippy::type_complexity,
    reason = "Contract tests keep compact fixture construction close to each assertion."
)]

use aqc_toml_engine_core as _;
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
    let current = current.map_or_else(Vec::new, |bytes| {
        vec![engine_core::EngineFileState {
            path: std::path::PathBuf::from("/workspace/Cargo.toml"),
            bytes: Some(bytes.to_vec()),
            executable: None,
        }]
    });
    cargo::CargoTomlEngine.reconcile(std::path::Path::new("/workspace"), &current, &reqs)
}

fn keyed_items<Entry: Default>(
    required: BTreeMap<String, (Entry, String)>,
    forbidden: BTreeMap<String, String>,
    closed: Option<String>,
) -> engine_core::ItemRequirements<engine_core::KeyedItem<Entry>> {
    engine_core::ItemRequirements {
        required: required
            .into_iter()
            .map(|(file_key, (value, msg))| (engine_core::KeyedItem { file_key, value }, msg))
            .collect(),
        forbidden: forbidden
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
fn package_scalar_equals_writes_on_empty() {
    let mut req = cargo::CargoTomlRequirements::default();
    let _ = req.package_fields.insert(
        "edition".to_owned(),
        cargo::PackageFieldAssertion::Scalar(engine_core::ScalarAssertion::Equals(
            engine_core::ConfigScalar::Str("2024".to_owned()),
            "edition".to_owned(),
        )),
    );
    let out = output(req, None);
    let text =
        String::from_utf8(first_bytes(&out)).expect("engine output should be valid UTF-8 TOML");
    assert!(text.contains("[package]"));
    assert!(text.contains("edition = \"2024\""));
}

#[test]
fn malformed_toml_returns_only_parse_error_and_no_expected_bytes() {
    let mut req = cargo::CargoTomlRequirements::default();
    let _ = req.package_fields.insert(
        "edition".to_owned(),
        cargo::PackageFieldAssertion::Scalar(engine_core::ScalarAssertion::Equals(
            engine_core::ConfigScalar::Str("2024".to_owned()),
            "edition".to_owned(),
        )),
    );

    let out = output(req, Some(b"[package\nname = \"fixture\"\n"));

    assert!(
        first_bytes(&out).is_empty(),
        "parse failures must not produce replacement bytes"
    );
    assert_eq!(out.findings.len(), 1, "parse failures must not cascade");
    assert!(
        matches!(
            out.findings.first(),
            Some(engine_core::Finding::ParseError { .. })
        ),
        "malformed Cargo.toml should report one parse error"
    );
}

#[test]
fn package_one_of_is_check_only_on_empty() {
    let mut allowed = BTreeSet::new();
    let _ = allowed.insert(engine_core::ConfigScalar::Str("MIT".to_owned()));
    let mut req = cargo::CargoTomlRequirements::default();
    let _ = req.package_fields.insert(
        "license".to_owned(),
        cargo::PackageFieldAssertion::Scalar(engine_core::ScalarAssertion::OneOf(
            allowed,
            "license".to_owned(),
        )),
    );
    let out = output(req, None);
    assert!(out.findings.iter().any(
        |f| matches!(f, engine_core::Finding::Mismatch { key, .. } if key == "[package].license")
    ));
}

#[test]
fn cargo_field_wrappers_reject_invalid_scalar_operations() {
    let mut edition_req = cargo::CargoTomlRequirements::default();
    let _ = edition_req.package_fields.insert(
        "edition".to_owned(),
        cargo::PackageFieldAssertion::Scalar(engine_core::ScalarAssertion::AtLeast(
            engine_core::ConfigScalar::Str("2024".to_owned()),
            "edition".to_owned(),
        )),
    );

    let (_edition_merged, edition_conflicts) =
        cargo::CargoTomlRequirements::merge(vec![(prov(), edition_req)]);
    assert!(
        edition_conflicts
            .iter()
            .any(|conflict| conflict.reason == "scalar-operation-unsupported")
    );

    let mut rust_version_req = cargo::CargoTomlRequirements::default();
    let _ = rust_version_req.package_fields.insert(
        "rust-version".to_owned(),
        cargo::PackageFieldAssertion::Scalar(engine_core::ScalarAssertion::Equals(
            engine_core::ConfigScalar::Str("1.85".to_owned()),
            "rust version".to_owned(),
        )),
    );
    let (_rust_version_merged, rust_version_conflicts) =
        cargo::CargoTomlRequirements::merge(vec![(prov(), rust_version_req)]);
    assert!(
        rust_version_conflicts
            .iter()
            .any(|conflict| conflict.reason == "scalar-operation-unsupported")
    );

    let mut workspace_members_req = cargo::CargoTomlRequirements::default();
    let _ = workspace_members_req.workspace_fields.insert(
        "members".to_owned(),
        cargo::WorkspaceFieldAssertion::Scalar(engine_core::ScalarAssertion::Equals(
            engine_core::ConfigScalar::Str("crate".to_owned()),
            "members".to_owned(),
        )),
    );
    let (_workspace_members_merged, workspace_members_conflicts) =
        cargo::CargoTomlRequirements::merge(vec![(prov(), workspace_members_req)]);
    assert!(
        workspace_members_conflicts
            .iter()
            .any(|conflict| conflict.reason == "scalar-operation-unsupported")
    );

    let mut target_fields_req = cargo::CargoTomlRequirements::default();
    let _ = target_fields_req.targets.lib_fields.insert(
        "path".to_owned(),
        cargo::TargetFieldAssertion::List(engine_core::ListRequirements::default()),
    );
    let _ = target_fields_req.targets.lib_fields.insert(
        "required-features".to_owned(),
        cargo::TargetFieldAssertion::Scalar(engine_core::ScalarAssertion::Equals(
            engine_core::ConfigScalar::Str("feature".to_owned()),
            "feature".to_owned(),
        )),
    );
    let (_target_fields_merged, target_fields_conflicts) =
        cargo::CargoTomlRequirements::merge(vec![(prov(), target_fields_req)]);
    assert_eq!(
        target_fields_conflicts
            .iter()
            .filter(|conflict| conflict.reason == "scalar-operation-unsupported")
            .count(),
        2
    );
}

#[test]
fn cargo_field_wrappers_reject_more_invalid_domain_shapes() {
    let mut req = cargo::CargoTomlRequirements::default();
    let _ = req.package_fields.insert(
        "rust-version".to_owned(),
        cargo::PackageFieldAssertion::OrderedVersion(engine_core::ScalarAssertion::AtMost(
            engine_core::DottedVersion::new("1.85"),
            "ceiling".to_owned(),
        )),
    );
    let _ = req.package_fields.insert(
        "version".to_owned(),
        cargo::PackageFieldAssertion::OrderedVersion(engine_core::ScalarAssertion::Range(
            engine_core::DottedVersion::new("1.0"),
            engine_core::DottedVersion::new("2.0"),
            "range".to_owned(),
        )),
    );
    let _ = req.package_fields.insert(
        "license".to_owned(),
        cargo::PackageFieldAssertion::OrderedVersion(engine_core::ScalarAssertion::AtLeast(
            engine_core::DottedVersion::new("1.0"),
            "not a version".to_owned(),
        )),
    );
    let _ = req.workspace_package_fields.insert(
        "license".to_owned(),
        cargo::PackageFieldAssertion::InheritsWorkspace("nested inherit".to_owned()),
    );
    let _ = req.workspace_fields.insert(
        "members".to_owned(),
        cargo::WorkspaceFieldAssertion::Scalar(engine_core::ScalarAssertion::OneOf(
            BTreeSet::from([engine_core::ConfigScalar::Str("crate".to_owned())]),
            "members oneof".to_owned(),
        )),
    );
    let _ = req.workspace_fields.insert(
        "resolver".to_owned(),
        cargo::WorkspaceFieldAssertion::Scalar(engine_core::ScalarAssertion::OneOf(
            BTreeSet::from([engine_core::ConfigScalar::Int(2)]),
            "wrong type".to_owned(),
        )),
    );
    let _ = req.targets.lib_fields.insert(
        "required-features".to_owned(),
        cargo::TargetFieldAssertion::Scalar(engine_core::ScalarAssertion::OneOf(
            BTreeSet::from([engine_core::ConfigScalar::Str("feature".to_owned())]),
            "features oneof".to_owned(),
        )),
    );

    let (_merged, conflicts) = cargo::CargoTomlRequirements::merge(vec![(prov(), req)]);
    assert_eq!(
        conflicts
            .iter()
            .filter(|conflict| conflict.reason == "scalar-operation-unsupported")
            .count(),
        7
    );
}

#[test]
fn cargo_package_workspace_inheritance_is_allowed_for_named_fields() {
    let mut req = cargo::CargoTomlRequirements::default();
    for field in ["license", "keywords", "categories"] {
        let _ = req.package_fields.insert(
            field.to_owned(),
            cargo::PackageFieldAssertion::InheritsWorkspace("workspace".to_owned()),
        );
    }

    let (_merged, conflicts) = cargo::CargoTomlRequirements::merge(vec![(prov(), req)]);
    assert!(conflicts.is_empty());
}

#[test]
fn package_lints_inherit_writes_workspace_true() {
    let mut req = cargo::CargoTomlRequirements::default();
    req.package_lints = Some(cargo::PackageLintsAssertion::Inherit(
        true,
        "inherit lints".to_owned(),
    ));
    let out = output(req, None);
    let text =
        String::from_utf8(first_bytes(&out)).expect("engine output should be valid UTF-8 TOML");
    assert!(text.contains("[lints]"));
    assert!(text.contains("workspace = true"));
}

#[test]
fn inline_lint_table_writes_one_lint() {
    let table: engine_core::ItemRequirements<engine_core::KeyedItem<cargo::LintSetting>> =
        keyed_items(
            BTreeMap::from([(
                "unwrap_used".to_owned(),
                (
                    cargo::LintSetting {
                        level: "deny".to_owned(),
                        priority: None,
                    },
                    "outer".to_owned(),
                ),
            )]),
            BTreeMap::new(),
            None,
        );
    let mut req = cargo::CargoTomlRequirements::default();
    req.package_lints = Some(cargo::PackageLintsAssertion::Inline(BTreeMap::from([(
        "clippy".to_owned(),
        table,
    )])));
    let out = output(req, None);
    let text =
        String::from_utf8(first_bytes(&out)).expect("engine output should be valid UTF-8 TOML");
    assert!(text.contains("[lints.clippy]"));
    assert!(text.contains("unwrap_used = \"deny\""));
}

#[test]
fn workspace_field_list_uses_unified_list_requirements() {
    let mut req = cargo::CargoTomlRequirements::default();
    let _ = req.workspace_fields.insert(
        "members".to_owned(),
        cargo::WorkspaceFieldAssertion::List(engine_core::ListRequirements {
            contains: BTreeMap::from([("crates/*".to_owned(), "members".to_owned())]),
            excludes: BTreeMap::new(),
            exact: None,
        }),
    );
    let out = output(req, None);
    let text =
        String::from_utf8(first_bytes(&out)).expect("engine output should be valid UTF-8 TOML");
    assert!(!out.findings.is_empty());
    assert!(text.contains("[workspace]"));
    assert!(text.contains("members = [\"crates/*\"]"));
}

#[test]
fn section_presence_absent_removes_table() {
    let mut req = cargo::CargoTomlRequirements::default();
    let _ = req.section_presence.insert(
        cargo::ManifestSection::Badges,
        cargo::SectionPresenceAssertion::Absent("no badges".to_owned()),
    );
    let out = output(
        req,
        Some(b"[badges]\nmaintenance = { status = \"actively-developed\" }\n"),
    );
    let text =
        String::from_utf8(first_bytes(&out)).expect("engine output should be valid UTF-8 TOML");
    assert!(!out.findings.is_empty());
    assert!(!text.contains("[badges]"));
}

#[test]
fn section_presence_writes_workspace_lints_table() {
    let mut req = cargo::CargoTomlRequirements::default();
    let _ = req.section_presence.insert(
        cargo::ManifestSection::WorkspaceLints,
        cargo::SectionPresenceAssertion::Present("workspace lints root".to_owned()),
    );
    let out = output(req, Some(b"[workspace]\nresolver = \"3\"\n"));
    let text =
        String::from_utf8(first_bytes(&out)).expect("engine output should be valid UTF-8 TOML");
    assert!(!out.findings.is_empty());
    assert!(text.contains("[workspace.lints]"));
}

fn first_bytes(output: &aqc_file_engine_core::EngineOutput) -> Vec<u8> {
    output
        .files
        .first()
        .map_or_else(Vec::new, |file| file.expected_bytes.clone())
}
