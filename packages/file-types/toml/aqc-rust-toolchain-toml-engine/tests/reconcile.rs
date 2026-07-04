use std::collections::BTreeMap;

use aqc_file_engine_core::{
    ConfigScalar, FileEngine, Finding, ListRequirements, Provenance, ScalarAssertion,
};
use aqc_rust_toolchain_toml_engine::{
    ResolvedRustToolchainTomlRequirements, RustToolchainListSetting, RustToolchainScalarSetting,
    RustToolchainTomlEngine, RustToolchainTomlRequirements,
};
use aqc_toml_engine_core as _;
use toml_edit as _;

#[test]
fn missing_file() {
    let output = reconcile_none(req_channel("stable"));

    assert!(
        String::from_utf8(output.expected_bytes)
            .unwrap_or_default()
            .contains("[toolchain]"),
        "missing file should be initialized"
    );
    assert_eq!(output.findings.len(), 2, "missing table and channel report");
}

#[test]
fn writes_deterministic_file() {
    let output = reconcile_none(req_components(vec!["rustfmt", "clippy"]));
    let expected = String::from_utf8(output.expected_bytes).unwrap_or_default();

    assert!(
        expected.contains("components = [\"clippy\", \"rustfmt\"]"),
        "components should be sorted deterministically"
    );
}

#[test]
fn missing_toolchain_table() {
    let output = reconcile("", req_channel("stable"));

    assert!(
        output
            .findings
            .iter()
            .any(|finding| matches!(finding, Finding::Mismatch { key, .. } if key == "toolchain")),
        "missing [toolchain] should report"
    );
}

#[test]
fn wrong_channel() {
    let output = reconcile(
        "[toolchain]\nchannel = \"nightly\"\n",
        req_channel("stable"),
    );

    assert!(
        output.findings.iter().any(
            |finding| matches!(finding, Finding::Mismatch { key, .. } if key == "toolchain.channel")
        ),
        "wrong channel should report"
    );
}

#[test]
fn missing_component() {
    let output = reconcile(
        "[toolchain]\nchannel = \"stable\"\ncomponents = [\"clippy\"]\n",
        req_components(vec!["clippy", "rustfmt"]),
    );

    assert!(
        output.findings.iter().any(
            |finding| matches!(finding, Finding::Mismatch { key, .. } if key == "toolchain.components.rustfmt")
        ),
        "missing component should report"
    );
}

#[test]
fn channel_and_path_conflict() {
    let output = reconcile(
        "[toolchain]\nchannel = \"stable\"\npath = \"/opt/rust\"\n",
        req_channel("stable"),
    );

    assert!(
        output.findings.iter().any(
            |finding| matches!(finding, Finding::Mismatch { key, .. } if key == "toolchain.path")
        ),
        "channel plus path should report"
    );
}

#[test]
fn path_blocks_components() {
    let output = reconcile(
        "[toolchain]\npath = \"/opt/rust\"\n",
        req_components(vec!["clippy"]),
    );

    assert!(
        output.findings.iter().any(
            |finding| matches!(finding, Finding::Mismatch { key, .. } if key == "toolchain.path")
        ),
        "path should block component requirements"
    );
}

#[test]
fn path_blocks_targets() {
    let output = reconcile(
        "[toolchain]\npath = \"/opt/rust\"\n",
        req_targets(vec!["x86_64-unknown-linux-gnu"]),
    );

    assert!(
        output.findings.iter().any(
            |finding| matches!(finding, Finding::Mismatch { key, .. } if key == "toolchain.path")
        ),
        "path should block target requirements"
    );
}

#[test]
fn path_blocks_profile() {
    let output = reconcile(
        "[toolchain]\npath = \"/opt/rust\"\n",
        req_profile("minimal"),
    );

    assert!(
        output.findings.iter().any(
            |finding| matches!(finding, Finding::Mismatch { key, .. } if key == "toolchain.path")
        ),
        "path should block profile requirements"
    );
}

#[test]
fn path_requirement_skips_channel_based_writes() {
    let mut req = req_path("/opt/rust");
    let _ = req.scalar_settings.insert(
        RustToolchainScalarSetting::Channel,
        ScalarAssertion::Equals(ConfigScalar::Str("stable".to_owned()), "channel".to_owned()),
    );
    req.list_settings = req_components(vec!["clippy"]).list_settings;

    let output = reconcile_none(req);
    let expected = String::from_utf8(output.expected_bytes).unwrap_or_default();

    assert!(
        output
            .findings
            .iter()
            .any(|finding| matches!(finding, Finding::InvalidRequirements { key, .. } if key == "toolchain.path")),
        "path plus channel-based requirements should report invalid requirements"
    );
    assert!(
        expected.contains("path = \"/opt/rust\""),
        "valid path requirement should still write path"
    );
    assert!(
        !expected.contains("channel") && !expected.contains("components"),
        "invalid channel-based settings must not be written beside path"
    );
}

#[test]
fn path_absent_requirement_allows_channel() {
    let output = reconcile(
        "[toolchain]\npath = \"/opt/rust\"\n",
        RustToolchainTomlRequirements {
            scalar_settings: BTreeMap::from([
                (
                    RustToolchainScalarSetting::Path,
                    ScalarAssertion::Absent("path must not be used".to_owned()),
                ),
                (
                    RustToolchainScalarSetting::Channel,
                    ScalarAssertion::Equals(
                        ConfigScalar::Str("stable".to_owned()),
                        "channel".to_owned(),
                    ),
                ),
            ]),
            ..RustToolchainTomlRequirements::default()
        },
    );
    let expected = String::from_utf8(output.expected_bytes).unwrap_or_default();

    assert!(
        !output
            .findings
            .iter()
            .any(|finding| matches!(finding, Finding::InvalidRequirements { .. })),
        "requiring path absent is compatible with channel"
    );
    assert!(
        expected.contains("channel = \"stable\"") && !expected.contains("path"),
        "path should be removed and channel should be written"
    );
}

#[test]
fn relative_path() {
    let output = reconcile(
        "[toolchain]\npath = \"relative\"\n",
        RustToolchainTomlRequirements::default(),
    );

    assert!(
        output.findings.iter().any(
            |finding| matches!(finding, Finding::Mismatch { key, expected, .. } if key == "toolchain.path" && expected == "absolute path")
        ),
        "relative path should report"
    );
}

#[test]
fn relative_path_requirement_is_not_written() {
    let output = reconcile_none(req_path("relative"));
    let expected = String::from_utf8(output.expected_bytes).unwrap_or_default();

    assert!(
        output
            .findings
            .iter()
            .any(|finding| matches!(finding, Finding::InvalidRequirements { key, .. } if key == "toolchain.path")),
        "relative path requirement should report invalid requirements"
    );
    assert!(
        !expected.contains("path = \"relative\""),
        "relative path requirement must not be written"
    );
}

#[test]
fn invalid_profile() {
    let output = reconcile("[toolchain]\nprofile = \"huge\"\n", req_profile("minimal"));

    assert!(
        output.findings.iter().any(
            |finding| matches!(finding, Finding::Mismatch { key, .. } if key == "toolchain.profile")
        ),
        "invalid profile should report"
    );
}

#[test]
fn empty_toolchain_table() {
    let output = reconcile("[toolchain]\n", RustToolchainTomlRequirements::default());

    assert!(
        output
            .findings
            .iter()
            .any(|finding| matches!(finding, Finding::Mismatch { key, .. } if key == "toolchain")),
        "empty [toolchain] should report"
    );
}

#[test]
fn closed_settings() {
    let output = reconcile(
        "[toolchain]\nchannel = \"stable\"\nextra = true\n",
        RustToolchainTomlRequirements {
            scalar_settings: BTreeMap::from([(
                RustToolchainScalarSetting::Channel,
                ScalarAssertion::Equals(
                    ConfigScalar::Str("stable".to_owned()),
                    "channel".to_owned(),
                ),
            )]),
            closed_settings: Some("closed".to_owned()),
            ..RustToolchainTomlRequirements::default()
        },
    );

    assert!(
        output.findings.iter().any(
            |finding| matches!(finding, Finding::Mismatch { key, .. } if key == "toolchain.extra")
        ),
        "closed settings should report unknown fields"
    );
}

#[test]
fn unknown_setting_allowed_when_open() {
    let output = reconcile(
        "[toolchain]\nchannel = \"stable\"\nextra = true\n",
        req_channel("stable"),
    );

    assert!(
        output.findings.is_empty(),
        "open settings should allow unknown fields"
    );
}

#[test]
fn list_order_is_ignored() {
    let output = reconcile(
        "[toolchain]\ncomponents = [\"rustfmt\", \"clippy\"]\n",
        req_components(vec!["clippy", "rustfmt"]),
    );

    assert!(
        output.findings.is_empty(),
        "same list members in different order should pass"
    );
}

fn req_channel(channel: &str) -> RustToolchainTomlRequirements {
    RustToolchainTomlRequirements {
        scalar_settings: BTreeMap::from([(
            RustToolchainScalarSetting::Channel,
            ScalarAssertion::Equals(ConfigScalar::Str(channel.to_owned()), "channel".to_owned()),
        )]),
        ..RustToolchainTomlRequirements::default()
    }
}

fn req_profile(profile: &str) -> RustToolchainTomlRequirements {
    RustToolchainTomlRequirements {
        scalar_settings: BTreeMap::from([(
            RustToolchainScalarSetting::Profile,
            ScalarAssertion::Equals(ConfigScalar::Str(profile.to_owned()), "profile".to_owned()),
        )]),
        ..RustToolchainTomlRequirements::default()
    }
}

fn req_path(path: &str) -> RustToolchainTomlRequirements {
    RustToolchainTomlRequirements {
        scalar_settings: BTreeMap::from([(
            RustToolchainScalarSetting::Path,
            ScalarAssertion::Equals(ConfigScalar::Str(path.to_owned()), "path".to_owned()),
        )]),
        ..RustToolchainTomlRequirements::default()
    }
}

fn req_components(values: Vec<&str>) -> RustToolchainTomlRequirements {
    req_list(RustToolchainListSetting::Components, values)
}

fn req_targets(values: Vec<&str>) -> RustToolchainTomlRequirements {
    req_list(RustToolchainListSetting::Targets, values)
}

fn req_list(setting: RustToolchainListSetting, values: Vec<&str>) -> RustToolchainTomlRequirements {
    RustToolchainTomlRequirements {
        list_settings: BTreeMap::from([(
            setting,
            ListRequirements {
                contains: values
                    .into_iter()
                    .map(|value| (value.to_owned(), format!("requires {value}")))
                    .collect(),
                ..ListRequirements::default()
            },
        )]),
        ..RustToolchainTomlRequirements::default()
    }
}

fn reconcile(
    current: &str,
    req: RustToolchainTomlRequirements,
) -> aqc_file_engine_core::EngineOutput {
    reconcile_resolved(Some(current.as_bytes()), req)
}

fn reconcile_none(req: RustToolchainTomlRequirements) -> aqc_file_engine_core::EngineOutput {
    reconcile_resolved(None, req)
}

fn reconcile_resolved(
    current: Option<&[u8]>,
    req: RustToolchainTomlRequirements,
) -> aqc_file_engine_core::EngineOutput {
    let (resolved, conflicts) = RustToolchainTomlRequirements::merge(vec![(
        Provenance {
            policy: "test".to_owned(),
        },
        req,
    )]);
    assert!(
        conflicts.is_empty(),
        "single requirement should not conflict: {conflicts:?}"
    );
    <RustToolchainTomlEngine as FileEngine<ResolvedRustToolchainTomlRequirements>>::reconcile(
        current, &resolved,
    )
}
