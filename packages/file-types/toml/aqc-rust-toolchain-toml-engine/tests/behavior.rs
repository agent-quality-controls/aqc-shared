use aqc_file_engine_core::{
    FileEngine, Finding, ListRequirements, Provenance, ScalarAssertion, Severity,
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
    let (_resolved, conflicts) = RustToolchainTomlRequirements::merge(vec![
        (prov("a"), req_channel("stable")),
        (prov("b"), req_channel("nightly")),
    ]);

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
        output.expected_bytes.is_empty()
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
    let expected = String::from_utf8_lossy(&output.expected_bytes);

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
    let expected = String::from_utf8_lossy(&output.expected_bytes);

    assert!(
        expected.contains("components = [\"clippy\", \"rustfmt\"]")
            && expected.contains("targets = [\"wasm32-unknown-unknown\"]"),
        "{components} and {targets} must be canonical"
    );
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
    let channel = match parsed {
        Ok(value) => value,
        Err(_) => RustToolchainChannel::stable(),
    };
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
    let path = match parsed {
        Ok(value) => value,
        Err(_) => return RustToolchainTomlRequirements::default(),
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
    let (resolved, conflicts) = RustToolchainTomlRequirements::merge(vec![(prov("test"), req)]);
    assert!(conflicts.is_empty(), "test requirement must merge");
    <RustToolchainTomlEngine as FileEngine<ResolvedRustToolchainTomlRequirements>>::reconcile(
        current, &resolved,
    )
}

fn prov(policy: &str) -> Provenance {
    Provenance {
        policy: policy.to_owned(),
    }
}
