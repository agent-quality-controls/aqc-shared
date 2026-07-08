#![expect(
    clippy::field_reassign_with_default,
    clippy::indexing_slicing,
    reason = "Dependency glob tests keep compact fixtures and index known merged keys."
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
fn package_glob_forbid_catches_plain_dependency_key() {
    let out = cargo_output(
        Some(b"[dependencies]\nopenssl-sys = \"0.9\"\nserde = \"1\"\n"),
        vec![(prov("p1"), dep_glob_req(vec![("openssl-*", "no openssl")]))],
    );
    let text =
        String::from_utf8(first_bytes(&out)).expect("engine output should be valid UTF-8 TOML");
    assert_eq!(
        mismatch_count_for_key(&out.findings, "[dependencies].openssl-sys"),
        1
    );
    assert!(!text.contains("openssl-sys"));
    assert!(text.contains("serde = \"1\""));
}

#[test]
fn package_glob_forbid_catches_renamed_dependency_package() {
    let out = cargo_output(
        Some(b"[dependencies]\nssl = { package = \"openssl-sys\", version = \"0.9\" }\nserde = \"1\"\n"),
        vec![(prov("p1"), dep_glob_req(vec![("openssl-*", "no openssl")]))],
    );
    let text =
        String::from_utf8(first_bytes(&out)).expect("engine output should be valid UTF-8 TOML");
    assert_eq!(
        mismatch_count_for_key(&out.findings, "[dependencies].ssl"),
        1
    );
    assert!(!text.contains("ssl ="));
    assert!(text.contains("serde = \"1\""));
}

#[test]
fn package_glob_forbid_applies_to_workspace_dependencies() {
    let mut req = cargo::CargoTomlRequirements::default();
    req.forbidden_workspace_dependency_package_globs =
        Some(dependency_package_globs(vec![("openssl-*", "no openssl")]));
    let out = cargo_output(
        Some(b"[workspace.dependencies]\nopenssl-sys = \"0.9\"\nserde = \"1\"\n"),
        vec![(prov("p1"), req)],
    );
    let text =
        String::from_utf8(first_bytes(&out)).expect("engine output should be valid UTF-8 TOML");
    assert_eq!(
        mismatch_count_for_key(&out.findings, "[workspace.dependencies].openssl-sys"),
        1
    );
    assert!(!text.contains("openssl-sys"));
    assert!(text.contains("serde = \"1\""));
}

#[test]
fn package_glob_forbid_applies_to_patch_tables() {
    let mut req = cargo::CargoTomlRequirements::default();
    let _ = req.forbidden_patch_dependency_package_globs.insert(
        "crates-io".to_owned(),
        dependency_package_globs(vec![("openssl-*", "no openssl")]),
    );
    let out = cargo_output(
        Some(
            b"[patch.crates-io]\nssl = { package = \"openssl-sys\", path = \"../ssl\" }\nserde = { path = \"../serde\" }\n",
        ),
        vec![(prov("p1"), req)],
    );
    let text =
        String::from_utf8(first_bytes(&out)).expect("engine output should be valid UTF-8 TOML");
    assert_eq!(
        mismatch_count_for_key(&out.findings, "[patch.crates-io].ssl"),
        1
    );
    assert!(!text.contains("ssl ="));
    assert!(text.contains("serde = { path = \"../serde\" }"));
}

#[test]
fn package_glob_forbid_applies_to_target_dependency_scope() {
    let mut req = cargo::CargoTomlRequirements::default();
    let _ = req.forbidden_dependency_package_globs.insert(
        unix_scope(),
        dependency_package_globs(vec![("openssl-*", "no openssl")]),
    );
    let out = cargo_output(
        Some(b"[target.'cfg(unix)'.dependencies]\nopenssl-sys = \"0.9\"\nserde = \"1\"\n"),
        vec![(prov("p1"), req)],
    );
    let text =
        String::from_utf8(first_bytes(&out)).expect("engine output should be valid UTF-8 TOML");
    assert_eq!(
        mismatch_count_for_key(
            &out.findings,
            "[target.'cfg(unix)'.dependencies].openssl-sys"
        ),
        1
    );
    assert!(!text.contains("openssl-sys"));
    assert!(text.contains("serde = \"1\""));
}

#[test]
fn package_glob_forbid_catches_dependency_subtable() {
    let out = cargo_output(
        Some(b"[dependencies.openssl]\npackage = \"openssl-sys\"\nversion = \"0.9\"\n"),
        vec![(prov("p1"), dep_glob_req(vec![("openssl-*", "no openssl")]))],
    );
    let text =
        String::from_utf8(first_bytes(&out)).expect("engine output should be valid UTF-8 TOML");
    assert_eq!(
        mismatch_count_for_key(&out.findings, "[dependencies].openssl"),
        1
    );
    assert!(!text.contains("[dependencies.openssl]"));
}

#[test]
fn dependency_package_identity_forbid_catches_subtable() {
    let req = dep_item_req(engine_core::ItemRequirements {
        required: Vec::new(),
        forbidden: vec![(
            package_requirement("openssl-sys", None),
            "no openssl".to_owned(),
        )],
        closed: None,
    });
    let out = cargo_output(
        Some(b"[dependencies.openssl]\npackage = \"openssl-sys\"\nversion = \"0.9\"\n"),
        vec![(prov("p1"), req)],
    );
    let text =
        String::from_utf8(first_bytes(&out)).expect("engine output should be valid UTF-8 TOML");
    assert_eq!(
        mismatch_count_for_key(&out.findings, "[dependencies].openssl"),
        1
    );
    assert!(!text.contains("[dependencies.openssl]"));
}

#[test]
fn required_package_matching_glob_forbid_conflicts() {
    let exact = dep_item_req(engine_core::ItemRequirements {
        required: vec![(
            package_requirement("openssl-sys", Some("0.9")),
            "need openssl".to_owned(),
        )],
        forbidden: Vec::new(),
        closed: None,
    });
    let glob = dep_glob_req(vec![("openssl-*", "no openssl")]);
    let (_, conflicts) =
        cargo::CargoTomlRequirements::merge(vec![(prov("p1"), exact), (prov("p2"), glob)]);
    assert!(
        conflicts.iter().any(|conflict| {
            conflict.reason == "dependency-package-glob-forbids-required-package"
        })
    );
}

#[test]
fn required_package_matching_glob_forbid_does_not_write_dependency() {
    let exact = dep_item_req(engine_core::ItemRequirements {
        required: vec![(
            package_requirement("openssl-sys", Some("0.9")),
            "need openssl".to_owned(),
        )],
        forbidden: Vec::new(),
        closed: None,
    });
    let glob = dep_glob_req(vec![("openssl-*", "no openssl")]);
    let out = cargo_output(None, vec![(prov("p1"), exact), (prov("p2"), glob)]);
    let text =
        String::from_utf8(first_bytes(&out)).expect("engine output should be valid UTF-8 TOML");
    assert!(has_conflict(&out.findings));
    assert!(!text.contains("openssl-sys"));
}

#[test]
fn required_local_key_matching_glob_forbid_does_not_remove_dependency() {
    let exact = dep_item_req(engine_core::ItemRequirements {
        required: vec![(
            local_dependency_requirement("ssl", Some("openssl-sys"), Some("0.9")),
            "need openssl".to_owned(),
        )],
        forbidden: Vec::new(),
        closed: None,
    });
    let glob = dep_glob_req(vec![("openssl-*", "no openssl")]);
    let out = cargo_output(
        Some(b"[dependencies]\nssl = { package = \"openssl-sys\", version = \"0.9\" }\n"),
        vec![(prov("p1"), exact), (prov("p2"), glob)],
    );
    let text =
        String::from_utf8(first_bytes(&out)).expect("engine output should be valid UTF-8 TOML");
    assert!(has_conflict(&out.findings));
    assert_eq!(
        mismatch_count_for_key(&out.findings, "[dependencies].ssl"),
        0
    );
    assert!(text.contains("ssl = { package = \"openssl-sys\", version = \"0.9\" }"));
}

#[test]
fn required_closed_dependency_matching_glob_forbid_does_not_remove_dependency() {
    let exact = dep_item_req(engine_core::ItemRequirements {
        required: vec![(
            local_dependency_requirement("ssl", Some("openssl-sys"), Some("0.9")),
            "need openssl".to_owned(),
        )],
        forbidden: Vec::new(),
        closed: Some("only declared deps".to_owned()),
    });
    let glob = dep_glob_req(vec![("openssl-*", "no openssl")]);
    let out = cargo_output(
        Some(b"[dependencies]\nssl = { package = \"openssl-sys\", version = \"0.9\" }\n"),
        vec![(prov("p1"), exact), (prov("p2"), glob)],
    );
    let text =
        String::from_utf8(first_bytes(&out)).expect("engine output should be valid UTF-8 TOML");
    assert!(has_conflict(&out.findings));
    assert_eq!(
        mismatch_count_for_key(&out.findings, "[dependencies].ssl"),
        0
    );
    assert!(text.contains("ssl = { package = \"openssl-sys\", version = \"0.9\" }"));
}

#[test]
fn same_forbidden_package_glob_dedupes_attribution() {
    let left = dep_glob_req(vec![("openssl-*", "left")]);
    let right = dep_glob_req(vec![("openssl-*", "right")]);
    let (merged, conflicts) =
        cargo::CargoTomlRequirements::merge(vec![(prov("p1"), left), (prov("p2"), right)]);
    assert!(conflicts.is_empty());
    let glob = &merged.forbidden_dependency_package_globs[&normal_scope()].globs["openssl-*"];
    assert_eq!(glob.collected.len(), 2);
}

#[test]
fn exact_forbidden_dependency_and_forbidden_glob_remove_dependency_once() {
    let mut req = dep_item_req(engine_core::ItemRequirements {
        required: Vec::new(),
        forbidden: vec![(
            package_requirement("openssl-sys", None),
            "exact no openssl".to_owned(),
        )],
        closed: None,
    });
    let _ = req.forbidden_dependency_package_globs.insert(
        normal_scope(),
        dependency_package_globs(vec![("openssl-*", "glob no openssl")]),
    );
    let out = cargo_output(
        Some(b"[dependencies]\nopenssl-sys = \"0.9\"\n"),
        vec![(prov("p1"), req)],
    );
    let text =
        String::from_utf8(first_bytes(&out)).expect("engine output should be valid UTF-8 TOML");
    assert_eq!(
        mismatch_count_for_key(&out.findings, "[dependencies].openssl-sys"),
        1
    );
    assert!(out.findings.iter().any(|finding| {
        matches!(
            finding,
            engine_core::Finding::Mismatch { attribution, .. } if attribution.len() == 1
        )
    }));
    assert!(!text.contains("openssl-sys"));
}

#[test]
fn closed_collection_and_forbidden_glob_remove_dependency_once() {
    let mut req = dep_item_req(engine_core::ItemRequirements {
        required: Vec::new(),
        forbidden: Vec::new(),
        closed: Some("closed".to_owned()),
    });
    let _ = req.forbidden_dependency_package_globs.insert(
        normal_scope(),
        dependency_package_globs(vec![("openssl-*", "glob no openssl")]),
    );
    let out = cargo_output(
        Some(b"[dependencies]\nopenssl-sys = \"0.9\"\n"),
        vec![(prov("p1"), req)],
    );
    let text =
        String::from_utf8(first_bytes(&out)).expect("engine output should be valid UTF-8 TOML");
    assert_eq!(
        mismatch_count_for_key(&out.findings, "[dependencies].openssl-sys"),
        1
    );
    assert!(!text.contains("openssl-sys"));
}

#[test]
fn invalid_forbidden_package_glob_reports_invalid_requirements() {
    let findings = cargo_findings_with(
        Some(b"[dependencies]\nserde = \"1\"\n"),
        vec![(prov("p1"), dep_glob_req(vec![("[", "bad glob")]))],
    );
    assert!(findings.iter().any(|finding| {
        matches!(
            finding,
            engine_core::Finding::InvalidRequirements { key, .. } if key == "[dependencies].["
        )
    }));
}

#[test]
fn patch_package_identity_init_reports_unwritable_required_key() {
    let mut req = cargo::CargoTomlRequirements::default();
    let _ = req.patch.insert(
        "crates-io".to_owned(),
        engine_core::ItemRequirements {
            required: vec![(
                package_requirement("serde_json", Some("1")),
                "serde".to_owned(),
            )],
            forbidden: Vec::new(),
            closed: None,
        },
    );
    let findings = cargo_output(None, vec![(prov("p1"), req)]).findings;
    assert!(findings.iter().any(|finding| {
        matches!(finding, engine_core::Finding::UnwritableRequiredKey { key, .. } if key == "[patch.crates-io].serde_json")
    }));
}

#[test]
fn patch_requirement_with_file_key_writes_patch_entry() {
    let mut req = cargo::CargoTomlRequirements::default();
    let _ = req.patch.insert(
        "crates-io".to_owned(),
        engine_core::ItemRequirements {
            required: vec![(
                cargo::DependencyRequirement {
                    file_key: Some("serde_json".to_owned()),
                    value: cargo::DependencySpec {
                        path: Some("../serde_json".to_owned()),
                        ..cargo::DependencySpec::default()
                    },
                },
                "patch serde".to_owned(),
            )],
            forbidden: Vec::new(),
            closed: None,
        },
    );
    let out = cargo_output(None, vec![(prov("p1"), req)]);
    let text =
        String::from_utf8(first_bytes(&out)).expect("engine output should be valid UTF-8 TOML");
    assert!(text.contains("[patch.crates-io]"));
    assert!(text.contains("serde_json = { path = \"../serde_json\" }"));
}

fn first_bytes(output: &aqc_file_engine_core::EngineOutput) -> Vec<u8> {
    output
        .files
        .first()
        .map_or_else(Vec::new, |file| file.expected_bytes.clone())
}
