use std::collections::{BTreeMap, BTreeSet};

use aqc_cargo_toml_engine::{
    CargoTomlEngine, CargoTomlRequirements, LintSetting, ManifestSection, PackageFieldAssertion,
    PackageLintsAssertion, SectionPresenceAssertion, WorkspaceFieldAssertion,
};
use aqc_file_engine_core::{
    ConfigScalar, Engine, EngineOutput, EngineRequirement, Finding, ItemRequirements, KeyedItem,
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
fn package_scalar_equals_writes_on_empty() {
    let mut req = CargoTomlRequirements::default();
    let _ = req.package_fields.insert(
        "edition".to_owned(),
        PackageFieldAssertion::Equals(ConfigScalar::Str("2024".to_owned()), "edition".to_owned()),
    );
    let out = output(req, None);
    let text =
        String::from_utf8(out.expected_bytes).expect("engine output should be valid UTF-8 TOML");
    assert!(text.contains("[package]"));
    assert!(text.contains("edition = \"2024\""));
}

#[test]
fn package_one_of_is_check_only_on_empty() {
    let mut allowed = BTreeSet::new();
    let _ = allowed.insert("MIT".to_owned());
    let mut req = CargoTomlRequirements::default();
    let _ = req.package_fields.insert(
        "license".to_owned(),
        PackageFieldAssertion::OneOf(allowed, "license".to_owned()),
    );
    let out = output(req, None);
    assert!(
        out.findings
            .iter()
            .any(|f| matches!(f, Finding::Mismatch { key, .. } if key == "[package].license"))
    );
}

#[test]
fn package_lints_inherit_writes_workspace_true() {
    let mut req = CargoTomlRequirements::default();
    req.package_lints = Some(PackageLintsAssertion::Inherit(
        true,
        "inherit lints".to_owned(),
    ));
    let out = output(req, None);
    let text =
        String::from_utf8(out.expected_bytes).expect("engine output should be valid UTF-8 TOML");
    assert!(text.contains("[lints]"));
    assert!(text.contains("workspace = true"));
}

#[test]
fn inline_lint_table_writes_one_lint() {
    let table: ItemRequirements<KeyedItem<LintSetting>> = keyed_items(
        BTreeMap::from([(
            "unwrap_used".to_owned(),
            (
                LintSetting {
                    level: "deny".to_owned(),
                    priority: None,
                },
                "outer".to_owned(),
            ),
        )]),
        BTreeMap::new(),
        None,
    );
    let mut req = CargoTomlRequirements::default();
    req.package_lints = Some(PackageLintsAssertion::Inline(BTreeMap::from([(
        "clippy".to_owned(),
        table,
    )])));
    let out = output(req, None);
    let text =
        String::from_utf8(out.expected_bytes).expect("engine output should be valid UTF-8 TOML");
    assert!(text.contains("[lints.clippy]"));
    assert!(text.contains("unwrap_used = \"deny\""));
}

#[test]
fn workspace_field_list_uses_unified_list_requirements() {
    let mut req = CargoTomlRequirements::default();
    let _ = req.workspace_fields.insert(
        "members".to_owned(),
        WorkspaceFieldAssertion::List(ListRequirements {
            contains: BTreeMap::from([("crates/*".to_owned(), "members".to_owned())]),
            excludes: BTreeMap::new(),
            exact: None,
        }),
    );
    let out = output(req, None);
    let text =
        String::from_utf8(out.expected_bytes).expect("engine output should be valid UTF-8 TOML");
    assert!(!out.findings.is_empty());
    assert!(text.contains("[workspace]"));
    assert!(text.contains("members = [\"crates/*\"]"));
}

#[test]
fn section_presence_absent_removes_table() {
    let mut req = CargoTomlRequirements::default();
    let _ = req.section_presence.insert(
        ManifestSection::Badges,
        SectionPresenceAssertion::Absent("no badges".to_owned()),
    );
    let out = output(
        req,
        Some(b"[badges]\nmaintenance = { status = \"actively-developed\" }\n"),
    );
    let text =
        String::from_utf8(out.expected_bytes).expect("engine output should be valid UTF-8 TOML");
    assert!(!out.findings.is_empty());
    assert!(!text.contains("[badges]"));
}
