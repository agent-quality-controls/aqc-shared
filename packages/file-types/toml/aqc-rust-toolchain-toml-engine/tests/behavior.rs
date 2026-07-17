use aqc_file_engine_core::{
    FileEngine, Finding, ItemRequirements, KeyedItem, ListRequirements, Provenance,
    ScalarAssertion, Severity,
};
use aqc_rust_toolchain_toml_engine::{
    ResolvedRustToolchainTomlRequirements, RustToolchainChannel, RustToolchainPath,
    RustToolchainProfile, RustToolchainTomlEngine, RustToolchainTomlRequirements,
};
use aqc_toml_engine_core as _;
use serde as _;
use toml_edit as _;

#[test]
fn merge_rejects_conflicting_channels() {
    let channel = "channel";
    let conflicts = RustToolchainTomlRequirements::merge(vec![
        (prov("a"), req_channel("stable")),
        (prov("b"), req_channel("nightly")),
    ])
    .expect_err("conflicting channels must not expose a resolved root");

    let conflict = conflicts
        .iter()
        .any(|entry| entry.key == "toolchain.channel" && entry.reason == "scalar-disagree");
    assert!(conflict, "{channel} conflict must be reported");
}

#[test]
fn merge_rejects_path_with_channel_requirement() {
    let path = "path";
    let channel = "channel";
    let output = reconcile_none(req_path_with(|req| {
        req.channel = Some(ScalarAssertion::Equals(
            RustToolchainChannel::stable(),
            "channel".to_owned(),
        ));
    }));

    assert!(
        output.findings.iter().any(
            |finding| matches!(finding, Finding::InvalidRequirements { key, .. } if key == "toolchain.channel")
        ),
        "{path} plus {channel} must be invalid"
    );
}

#[test]
fn merge_rejects_path_with_profile_requirement() {
    let path = "path";
    let profile = "profile";
    let output = reconcile_none(req_path_with(|req| {
        req.profile = Some(ScalarAssertion::Equals(
            RustToolchainProfile::Minimal,
            "profile".to_owned(),
        ));
    }));

    assert!(
        output.findings.iter().any(
            |finding| matches!(finding, Finding::InvalidRequirements { key, .. } if key == "toolchain.profile")
        ),
        "{path} plus {profile} must be invalid"
    );
}

#[test]
fn merge_rejects_path_with_components_requirement() {
    let path = "path";
    let components = "components";
    let output = reconcile_none(req_path_with(|req| {
        req.components = contains(["clippy"]);
    }));

    assert!(
        output.findings.iter().any(
            |finding| matches!(finding, Finding::InvalidRequirements { key, .. } if key == "toolchain.components")
        ),
        "{path} plus {components} must be invalid"
    );
}

#[test]
fn merge_rejects_path_with_targets_requirement() {
    let path = "path";
    let targets = "targets";
    let output = reconcile_none(req_path_with(|req| {
        req.targets = contains(["wasm32-unknown-unknown"]);
    }));

    assert!(
        output.findings.iter().any(
            |finding| matches!(finding, Finding::InvalidRequirements { key, .. } if key == "toolchain.targets")
        ),
        "{path} plus {targets} must be invalid"
    );
}

#[test]
fn reconcile_refuses_malformed_toml() {
    let malformed = "[toolchain";
    let output = reconcile(malformed, baseline_req());

    assert!(
        first_bytes(&output).is_empty()
            && output.findings.iter().any(|finding| matches!(
                finding,
                Finding::ParseError {
                    severity: Severity::Error,
                    ..
                }
            )),
        "{malformed} must be refused"
    );
}

#[test]
fn reconcile_refuses_path_with_channel() {
    let path = "path";
    let channel = "channel";
    let output = reconcile(
        "[toolchain]\npath = \"/opt/rust\"\nchannel = \"stable\"\n",
        baseline_req(),
    );

    assert!(
        output.findings.iter().any(
            |finding| matches!(finding, Finding::UnwritableRequiredKey { key, .. } if key == "toolchain.path")
                || matches!(finding, Finding::Mismatch { key, .. } if key == "toolchain.path")
        ),
        "{path} plus {channel} must be refused"
    );
}

#[test]
fn reconcile_refuses_path_with_profile() {
    let path = "path";
    let profile = "profile";
    let output = reconcile(
        "[toolchain]\npath = \"/opt/rust\"\nprofile = \"minimal\"\n",
        baseline_req(),
    );

    assert!(
        output.findings.iter().any(
            |finding| matches!(finding, Finding::UnwritableRequiredKey { key, .. } if key == "toolchain.path")
        ),
        "{path} plus {profile} must be refused"
    );
}

#[test]
fn reconcile_refuses_path_with_components() {
    let path = "path";
    let components = "components";
    let output = reconcile(
        "[toolchain]\npath = \"/opt/rust\"\ncomponents = [\"clippy\"]\n",
        baseline_req(),
    );

    assert!(
        output.findings.iter().any(
            |finding| matches!(finding, Finding::UnwritableRequiredKey { key, .. } if key == "toolchain.path")
        ),
        "{path} plus {components} must be refused"
    );
}

#[test]
fn reconcile_refuses_path_with_targets() {
    let path = "path";
    let targets = "targets";
    let output = reconcile(
        "[toolchain]\npath = \"/opt/rust\"\ntargets = [\"wasm32-unknown-unknown\"]\n",
        baseline_req(),
    );

    assert!(
        output.findings.iter().any(
            |finding| matches!(finding, Finding::UnwritableRequiredKey { key, .. } if key == "toolchain.path")
        ),
        "{path} plus {targets} must be refused"
    );
}

#[test]
fn reconcile_refuses_path_instead_of_channel_policy() {
    let path = "path";
    let channel = "channel";
    let output = reconcile("[toolchain]\npath = \"/opt/rust\"\n", baseline_req());

    assert!(
        output.findings.iter().any(
            |finding| matches!(finding, Finding::UnwritableRequiredKey { key, .. } if key == "toolchain.path")
        ),
        "{path} cannot satisfy {channel} policy"
    );
}

#[test]
fn reconcile_repairs_missing_channel_profile_components() {
    let channel = "channel";
    let profile = "profile";
    let components = "components";
    let output = reconcile("[toolchain]\n", baseline_req());
    let expected_bytes = first_bytes(&output);
    let expected = String::from_utf8_lossy(&expected_bytes);

    assert!(
        expected.contains("channel = \"stable\"")
            && expected.contains("profile = \"minimal\"")
            && expected.contains("components = [\"clippy\", \"rustfmt\"]"),
        "{channel}, {profile}, and {components} must be written"
    );
}

#[test]
fn reconcile_canonicalizes_components_and_targets() {
    let components = "components";
    let targets = "targets";
    let output = reconcile(
        "[toolchain]\ncomponents = [\"rustfmt\", \"clippy\"]\ntargets = [\"wasm32-unknown-unknown\"]\n",
        RustToolchainTomlRequirements {
            components: contains(["clippy", "rustfmt"]),
            targets: contains(["wasm32-unknown-unknown"]),
            ..RustToolchainTomlRequirements::default()
        },
    );
    let expected_bytes = first_bytes(&output);
    let expected = String::from_utf8_lossy(&expected_bytes);

    assert!(
        expected.contains("components = [\"clippy\", \"rustfmt\"]")
            && expected.contains("targets = [\"wasm32-unknown-unknown\"]"),
        "{components} and {targets} must be canonical"
    );
}

#[test]
fn explicit_membership_reports_and_removes_unknown_toolchain_fields() {
    let output = reconcile(
        "[toolchain]\nchannel = \"stable\"\nfuture-setting = true\n",
        RustToolchainTomlRequirements {
            channel: Some(ScalarAssertion::Equals(
                RustToolchainChannel::stable(),
                "channel".to_owned(),
            )),
            toolchain_keys: ItemRequirements {
                allowed: None,
                exact: Some((
                    vec![KeyedItem {
                        file_key: "channel".to_owned(),
                        value: (),
                    }],
                    "exact settings".to_owned(),
                )),
                ..ItemRequirements::default()
            },
            ..RustToolchainTomlRequirements::default()
        },
    );
    let expected_bytes = first_bytes(&output);
    let expected = String::from_utf8_lossy(&expected_bytes);

    assert!(!expected.contains("future-setting"));
    assert!(output.findings.iter().any(|finding| {
        matches!(
            finding,
            Finding::Mismatch { key, message, .. }
                if key == "toolchain.future-setting" && message == "exact settings"
        )
    }));
}

#[test]
fn explicit_toolchain_membership_reports_missing_and_forbidden_keys() {
    let output = reconcile(
        "[toolchain]\nforbidden = true\n",
        RustToolchainTomlRequirements {
            toolchain_keys: ItemRequirements {
                required: vec![(toolchain_key("missing"), "required".to_owned())],
                forbidden: vec![(toolchain_key("forbidden"), "forbidden".to_owned())],
                ..ItemRequirements::default()
            },
            ..RustToolchainTomlRequirements::default()
        },
    );

    assert!(output.findings.iter().any(|finding| matches!(
        finding,
        Finding::UnwritableRequiredKey { key, .. } if key == "toolchain.missing"
    )));
    assert!(output.findings.iter().any(|finding| matches!(
        finding,
        Finding::Mismatch { key, message, .. }
            if key == "toolchain.forbidden" && message == "forbidden"
    )));
}

#[test]
fn exact_toolchain_membership_does_not_authorize_path_or_empty_targets() {
    let requirement = RustToolchainTomlRequirements {
        channel: Some(ScalarAssertion::Equals(
            RustToolchainChannel::stable(),
            "channel".to_owned(),
        )),
        toolchain_keys: exact_toolchain_keys(["channel"], "channel only"),
        ..RustToolchainTomlRequirements::default()
    };
    let output = reconcile(
        "[toolchain]\nchannel = \"stable\"\npath = \"/opt/rust\"\n",
        requirement.clone(),
    );
    assert!(
        !String::from_utf8(first_bytes(&output))
            .unwrap_or_default()
            .contains("path")
    );

    let initialized = reconcile_none(requirement);
    let rendered = String::from_utf8(first_bytes(&initialized)).unwrap_or_default();
    assert!(rendered.contains("channel = \"stable\""));
    assert!(!rendered.contains("targets"));
}

#[test]
fn exact_toolchain_membership_initializes_to_a_fixed_point() {
    let requirement = RustToolchainTomlRequirements {
        channel: Some(ScalarAssertion::Equals(
            RustToolchainChannel::stable(),
            "channel".to_owned(),
        )),
        toolchain_keys: exact_toolchain_keys(["channel"], "channel only"),
        ..RustToolchainTomlRequirements::default()
    };
    let resolved = RustToolchainTomlRequirements::merge(vec![(prov("policy"), requirement)])
        .expect("requirements must merge");
    let initialized = <RustToolchainTomlEngine as FileEngine<_>>::reconcile(None, &resolved);
    let second = <RustToolchainTomlEngine as FileEngine<_>>::reconcile(
        Some(&initialized.expected_bytes),
        &resolved,
    );

    assert!(second.findings.is_empty());
}

#[test]
fn conflicting_exact_toolchain_keys_fail_merge() {
    let requirement = |key_name: &str| RustToolchainTomlRequirements {
        toolchain_keys: exact_toolchain_keys([key_name], key_name),
        ..RustToolchainTomlRequirements::default()
    };
    let conflicts = RustToolchainTomlRequirements::merge(vec![
        (prov("one"), requirement("channel")),
        (prov("two"), requirement("path")),
    ])
    .expect_err("different exact toolchain keys must conflict");

    assert!(conflicts.iter().any(|conflict| conflict.key == "toolchain"));
}

#[test]
fn exact_empty_toolchain_membership_fails_merge() {
    let requirement = RustToolchainTomlRequirements {
        toolchain_keys: exact_toolchain_keys([], "no toolchain keys"),
        ..RustToolchainTomlRequirements::default()
    };
    let conflicts = RustToolchainTomlRequirements::merge(vec![(prov("policy"), requirement)])
        .expect_err("rust-toolchain.toml cannot contain an empty [toolchain] table");

    assert!(conflicts.iter().any(|conflict| {
        conflict.key == "toolchain" && conflict.reason == "empty-toolchain-table"
    }));
}

#[test]
fn exact_toolchain_keys_cannot_exclude_a_constructive_value_requirement() {
    let requirement = RustToolchainTomlRequirements {
        channel: Some(ScalarAssertion::Equals(
            RustToolchainChannel::stable(),
            "stable channel".to_owned(),
        )),
        toolchain_keys: exact_toolchain_keys([], "no toolchain keys"),
        ..RustToolchainTomlRequirements::default()
    };

    let conflicts = RustToolchainTomlRequirements::merge(vec![(prov("policy"), requirement)])
        .expect_err("value and membership requirements must conflict");

    assert!(
        conflicts
            .iter()
            .any(|conflict| conflict.key == "toolchain.channel")
    );
}

#[test]
fn exact_membership_removes_an_absent_scalar_before_child_reconciliation() {
    let output = reconcile(
        "[toolchain]\nchannel = \"stable\"\nprofile = \"minimal\"\n",
        RustToolchainTomlRequirements {
            profile: Some(ScalarAssertion::Absent("profile absent".to_owned())),
            toolchain_keys: exact_toolchain_keys(["channel"], "channel only"),
            ..RustToolchainTomlRequirements::default()
        },
    );

    assert!(
        !String::from_utf8(first_bytes(&output))
            .unwrap_or_default()
            .contains("profile")
    );
    assert!(output.findings.iter().any(|finding| matches!(
        finding,
        Finding::Mismatch { key, message, .. }
            if key == "toolchain.profile" && message == "channel only"
    )));
    assert!(!output.findings.iter().any(|finding| matches!(
        finding,
        Finding::Mismatch { key, message, .. }
            if key == "toolchain.profile" && message == "profile absent"
    )));
}

#[test]
fn toolchain_membership_findings_include_all_exact_contributors() {
    let requirement = || RustToolchainTomlRequirements {
        toolchain_keys: exact_toolchain_keys(["channel"], "channel only"),
        ..RustToolchainTomlRequirements::default()
    };
    let resolved = RustToolchainTomlRequirements::merge(vec![
        (prov("two"), requirement()),
        (prov("one"), requirement()),
    ])
    .expect("agreeing requirements must merge");
    let output = <RustToolchainTomlEngine as FileEngine<_>>::reconcile(
        Some(b"[toolchain]\nchannel = \"stable\"\nunknown = true\n"),
        &resolved,
    );

    assert!(output.findings.iter().any(|finding| matches!(
        finding,
        Finding::Mismatch { key, attribution, .. }
            if key == "toolchain.unknown"
                && attribution == &[prov("one"), prov("two")]
    )));
}

fn toolchain_key(file_key: &str) -> KeyedItem<()> {
    KeyedItem {
        file_key: file_key.to_owned(),
        value: (),
    }
}

fn exact_toolchain_keys<const N: usize>(
    keys: [&str; N],
    message: &str,
) -> ItemRequirements<KeyedItem<()>> {
    ItemRequirements {
        allowed: None,
        exact: Some((
            keys.into_iter().map(toolchain_key).collect(),
            message.to_owned(),
        )),
        ..ItemRequirements::default()
    }
}

#[test]
fn allowed_only_toolchain_keys_leave_an_absent_table_absent() {
    let requirement = RustToolchainTomlRequirements {
        toolchain_keys: ItemRequirements {
            allowed: Some((
                vec![toolchain_key("channel")],
                "channel is allowed".to_owned(),
            )),
            ..ItemRequirements::default()
        },
        ..RustToolchainTomlRequirements::default()
    };
    let resolved = RustToolchainTomlRequirements::merge(vec![(prov("policy"), requirement)])
        .expect("allowed-only keys must resolve");
    let output = <RustToolchainTomlEngine as FileEngine<_>>::reconcile(Some(b""), &resolved);

    assert!(output.findings.is_empty());
    assert!(output.expected_bytes.is_empty());
}

#[test]
fn nonconstructive_toolchain_key_membership_leaves_an_absent_table_absent() {
    for toolchain_keys in [
        ItemRequirements {
            forbidden: vec![(toolchain_key("path"), "path is forbidden".to_owned())],
            ..ItemRequirements::default()
        },
        ItemRequirements {
            forbidden: vec![(toolchain_key("path"), "path is forbidden".to_owned())],
            allowed: Some((
                vec![toolchain_key("channel")],
                "channel is allowed".to_owned(),
            )),
            ..ItemRequirements::default()
        },
    ] {
        let resolved = RustToolchainTomlRequirements::merge(vec![(
            prov("policy"),
            RustToolchainTomlRequirements {
                toolchain_keys,
                ..RustToolchainTomlRequirements::default()
            },
        )])
        .expect("nonconstructive key membership must resolve");
        let output = <RustToolchainTomlEngine as FileEngine<_>>::reconcile(Some(b""), &resolved);

        assert!(output.findings.is_empty());
        assert!(output.expected_bytes.is_empty());
    }
}

#[test]
fn exclusion_only_toolchain_lists_leave_an_absent_table_absent() {
    for requirement in [
        RustToolchainTomlRequirements {
            components: ListRequirements {
                excludes: [("rustfmt".to_owned(), "exclude rustfmt".to_owned())].into(),
                ..ListRequirements::default()
            },
            ..RustToolchainTomlRequirements::default()
        },
        RustToolchainTomlRequirements {
            targets: ListRequirements {
                excludes: [(
                    "wasm32-unknown-unknown".to_owned(),
                    "exclude target".to_owned(),
                )]
                .into(),
                ..ListRequirements::default()
            },
            ..RustToolchainTomlRequirements::default()
        },
    ] {
        let resolved = RustToolchainTomlRequirements::merge(vec![(prov("policy"), requirement)])
            .expect("exclusion-only lists must resolve");
        let output = <RustToolchainTomlEngine as FileEngine<_>>::reconcile(Some(b""), &resolved);

        assert!(output.findings.is_empty());
        assert!(output.expected_bytes.is_empty());
    }
}

#[test]
fn path_is_compatible_with_nonconstructive_channel_requirements() {
    let requirement = req_path_with(|requirement| {
        requirement.channel = Some(ScalarAssertion::Absent("no channel".to_owned()));
        requirement.profile = Some(ScalarAssertion::Absent("no profile".to_owned()));
        requirement.components = ListRequirements {
            excludes: [("rustfmt".to_owned(), "exclude rustfmt".to_owned())].into(),
            ..ListRequirements::default()
        };
        requirement.targets = ListRequirements {
            excludes: [(
                "wasm32-unknown-unknown".to_owned(),
                "exclude target".to_owned(),
            )]
            .into(),
            ..ListRequirements::default()
        };
    });
    let output = reconcile(
        "[toolchain]\npath = \"/opt/rust\"\nchannel = \"stable\"\nprofile = \"default\"\ncomponents = [\"rustfmt\", \"other\"]\ntargets = [\"wasm32-unknown-unknown\", \"other-target\"]\n",
        requirement.clone(),
    );
    let rendered = String::from_utf8(output.expected_bytes.clone())
        .expect("engine output should remain UTF-8 TOML");

    assert!(rendered.contains("path = \"/opt/rust\""));
    assert!(!rendered.contains("channel ="));
    assert!(!rendered.contains("profile ="));
    assert!(!rendered.contains("rustfmt"));
    assert!(!rendered.contains("wasm32-unknown-unknown"));
    assert!(rendered.contains("other"));
    let fixed = reconcile_bytes(Some(&output.expected_bytes), requirement);
    assert!(fixed.findings.is_empty());
    assert_eq!(fixed.expected_bytes, output.expected_bytes);
}

fn baseline_req() -> RustToolchainTomlRequirements {
    RustToolchainTomlRequirements {
        channel: Some(ScalarAssertion::Equals(
            RustToolchainChannel::stable(),
            "channel".to_owned(),
        )),
        profile: Some(ScalarAssertion::Equals(
            RustToolchainProfile::Minimal,
            "profile".to_owned(),
        )),
        components: contains(["clippy", "rustfmt"]),
        ..RustToolchainTomlRequirements::default()
    }
}

fn req_channel(value: &str) -> RustToolchainTomlRequirements {
    let parsed = RustToolchainChannel::new(value);
    assert!(parsed.is_ok(), "test channel must parse");
    let channel = parsed.unwrap_or_else(|_| RustToolchainChannel::stable());
    RustToolchainTomlRequirements {
        channel: Some(ScalarAssertion::Equals(channel, "channel".to_owned())),
        ..RustToolchainTomlRequirements::default()
    }
}

fn req_path_with(
    update: impl FnOnce(&mut RustToolchainTomlRequirements),
) -> RustToolchainTomlRequirements {
    let parsed = RustToolchainPath::new("/opt/rust");
    assert!(parsed.is_ok(), "test path must parse");
    let Ok(path) = parsed else {
        return RustToolchainTomlRequirements::default();
    };
    let mut req = RustToolchainTomlRequirements {
        path: Some(ScalarAssertion::Equals(path, "path".to_owned())),
        ..RustToolchainTomlRequirements::default()
    };
    update(&mut req);
    req
}

fn contains<const N: usize>(values: [&str; N]) -> ListRequirements {
    ListRequirements {
        contains: values
            .into_iter()
            .map(|value| (value.to_owned(), "required list value".to_owned()))
            .collect(),
        ..ListRequirements::default()
    }
}

fn reconcile_none(req: RustToolchainTomlRequirements) -> aqc_file_engine_core::EngineOutput {
    reconcile_bytes(None, req)
}

fn reconcile(
    current: &str,
    req: RustToolchainTomlRequirements,
) -> aqc_file_engine_core::EngineOutput {
    reconcile_bytes(Some(current.as_bytes()), req)
}

fn reconcile_bytes(
    current: Option<&[u8]>,
    req: RustToolchainTomlRequirements,
) -> aqc_file_engine_core::EngineOutput {
    let result = RustToolchainTomlRequirements::merge(vec![(prov("test"), req)]);
    assert!(result.is_ok(), "test requirement must merge");
    let resolved = result.unwrap_or_default();
    <RustToolchainTomlEngine as FileEngine<ResolvedRustToolchainTomlRequirements>>::reconcile(
        current, &resolved,
    )
}

fn prov(policy: &str) -> Provenance {
    Provenance {
        policy: policy.to_owned(),
    }
}

fn first_bytes(output: &aqc_file_engine_core::EngineOutput) -> Vec<u8> {
    output.expected_bytes.clone()
}
use schemars as _;
