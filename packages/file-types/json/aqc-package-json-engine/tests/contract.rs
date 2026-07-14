#![expect(
    clippy::expect_used,
    clippy::panic,
    reason = "The accepted contract requires this exact integration-test filename, and shared test helpers assert fixture invariants."
)]

use aqc_file_engine_core::{
    Engine, EngineRequirement, FileEngine, Finding, Provenance, ScalarAssertion,
};
use aqc_json_engine_core as _;
use aqc_package_json_engine::{
    DevEnginePackageManagerRequirements, PackageJsonEngine, PackageJsonRequirements,
    PackageManagerOnFail, ResolvedPackageJsonRequirements,
};
use schemars as _;
use serde as _;

const POLICY: &str = "consumer-policy";

#[test]
fn missing_package_json_creates_both_package_manager_declarations_deterministically() {
    let resolved = merge(vec![(provenance(POLICY), complete_requirement("11.11.0"))]);
    let output = <PackageJsonEngine as FileEngine<ResolvedPackageJsonRequirements>>::reconcile(
        None, &resolved,
    );
    assert_eq!(
        output.expected_bytes,
        b"{\n  \"devEngines\": {\n    \"packageManager\": {\n      \"name\": \"pnpm\",\n      \"onFail\": \"error\",\n      \"version\": \"11.11.0\"\n    }\n  },\n  \"packageManager\": \"pnpm@11.11.0\"\n}\n",
        "Missing package.json creation must be canonical and complete."
    );
    assert_eq!(
        output.findings.len(),
        4,
        "Each initially missing declaration must be reported."
    );
}

#[test]
fn valid_existing_package_json_is_an_exact_no_op() {
    let bytes = b"{\n  \"name\": \"kept\",\n  \"packageManager\": \"pnpm@11.11.0\",\n  \"devEngines\": {\"packageManager\": {\"name\": \"pnpm\", \"version\": \"11.11.0\", \"onFail\": \"error\"}}\n}\n";
    let output = reconcile_existing(bytes);
    assert_eq!(
        output.expected_bytes, bytes,
        "A valid existing file must preserve exact bytes."
    );
    assert!(
        output.findings.is_empty(),
        "A valid existing file must have no findings."
    );
}

#[test]
fn existing_empty_object_returns_corrected_expected_bytes_and_findings() {
    let output = reconcile_existing(b"{}\n");
    assert_eq!(
        output.expected_bytes,
        b"{\n  \"devEngines\": {\n    \"packageManager\": {\n      \"name\": \"pnpm\",\n      \"onFail\": \"error\",\n      \"version\": \"11.11.0\"\n    }\n  },\n  \"packageManager\": \"pnpm@11.11.0\"\n}\n",
        "A writable mismatch must produce corrected expected bytes."
    );
    assert_eq!(
        output.findings.len(),
        4,
        "Every missing declaration must still be reported."
    );
}

#[test]
fn invalid_existing_package_json_returns_corrected_bytes_and_findings() {
    let bytes = br#"{"packageManager":"npm@10","devEngines":{"packageManager":{"name":"npm","version":"10","onFail":"warn"}}}"#;
    let output = reconcile_existing(bytes);
    assert_eq!(
        output.expected_bytes,
        b"{\n  \"devEngines\": {\n    \"packageManager\": {\n      \"name\": \"pnpm\",\n      \"onFail\": \"error\",\n      \"version\": \"11.11.0\"\n    }\n  },\n  \"packageManager\": \"pnpm@11.11.0\"\n}\n",
        "Writable mismatches must produce corrected expected bytes."
    );
    assert_eq!(
        output.findings.len(),
        4,
        "Every invalid declaration must be reported."
    );
    for finding in output.findings {
        let Finding::Mismatch { attribution, .. } = finding else {
            panic!("Invalid declarations must produce mismatch findings.");
        };
        assert_eq!(
            attribution,
            vec![provenance(POLICY)],
            "Every package mismatch must preserve policy provenance."
        );
    }
}

#[test]
fn nested_writes_preserve_unaddressed_values() {
    let bytes = br#"{"name":"kept","devEngines":{"kept":true,"packageManager":{"name":"npm","custom":42}}}"#;
    let output = reconcile_existing(bytes);
    assert_eq!(
        output.expected_bytes,
        b"{\n  \"devEngines\": {\n    \"kept\": true,\n    \"packageManager\": {\n      \"custom\": 42,\n      \"name\": \"pnpm\",\n      \"onFail\": \"error\",\n      \"version\": \"11.11.0\"\n    }\n  },\n  \"name\": \"kept\",\n  \"packageManager\": \"pnpm@11.11.0\"\n}\n",
        "Nested corrections must preserve every unaddressed value."
    );
    assert_eq!(
        output.findings.len(),
        4,
        "Every missing or invalid managed field must be reported."
    );
}

#[test]
fn wrong_container_shapes_are_replaced_by_corrected_nested_objects() {
    let bytes = br#"{"packageManager":{},"devEngines":{"packageManager":[]},"private":true}"#;
    let output = reconcile_existing(bytes);
    assert_eq!(
        output.expected_bytes,
        b"{\n  \"devEngines\": {\n    \"packageManager\": {\n      \"name\": \"pnpm\",\n      \"onFail\": \"error\",\n      \"version\": \"11.11.0\"\n    }\n  },\n  \"packageManager\": \"pnpm@11.11.0\",\n  \"private\": true\n}\n",
        "Writable assertions must replace wrong managed shapes without changing unmanaged values."
    );
    assert_eq!(
        output.findings.len(),
        4,
        "Each wrong or missing managed scalar must be reported."
    );
}

#[test]
fn wrong_managed_leaf_shapes_are_replaced_independently() {
    for value in ["true", "42", "null", "[]", "{}"] {
        let bytes = r#"{"packageManager":VALUE,"devEngines":{"packageManager":{"name":VALUE,"version":VALUE,"onFail":VALUE}}}"#
            .replace("VALUE", value);
        let output = reconcile_existing(bytes.as_bytes());
        assert_eq!(output.findings.len(), 4, "wrong shape {value}");
        let fixed = <PackageJsonEngine as FileEngine<ResolvedPackageJsonRequirements>>::reconcile(
            Some(&output.expected_bytes),
            &resolved_complete(),
        );
        assert!(fixed.findings.is_empty(), "wrong shape {value}");
    }
}

#[test]
fn absent_assertion_removes_only_the_addressed_value() {
    let resolved = merge(vec![(
        provenance(POLICY),
        PackageJsonRequirements {
            package_manager: Some(ScalarAssertion::Absent("Remove packageManager.".to_owned())),
            dev_engines_package_manager: DevEnginePackageManagerRequirements::default(),
            ..PackageJsonRequirements::default()
        },
    )]);
    let bytes = br#"{"name":"kept","packageManager":{},"private":true}"#;
    let output = <PackageJsonEngine as FileEngine<ResolvedPackageJsonRequirements>>::reconcile(
        Some(bytes),
        &resolved,
    );
    assert_eq!(
        output.findings.len(),
        1,
        "A present object must not satisfy a scalar absence assertion."
    );
    assert_eq!(
        output.expected_bytes, b"{\n  \"name\": \"kept\",\n  \"private\": true\n}\n",
        "Absence must remove the addressed field and preserve other values."
    );
}

#[test]
fn check_only_assertions_report_without_inventing_values() {
    let resolved = merge(vec![(
        provenance(POLICY),
        PackageJsonRequirements {
            package_manager: Some(ScalarAssertion::Present(
                "Declare a package manager.".to_owned(),
            )),
            dev_engines_package_manager: DevEnginePackageManagerRequirements {
                name: Some(ScalarAssertion::OneOf(
                    ["pnpm".to_owned(), "npm".to_owned()].into_iter().collect(),
                    "Use a supported package manager.".to_owned(),
                )),
                ..DevEnginePackageManagerRequirements::default()
            },
            ..PackageJsonRequirements::default()
        },
    )]);
    let bytes = b"{ \"name\": \"kept\" }\n";
    let output = <PackageJsonEngine as FileEngine<ResolvedPackageJsonRequirements>>::reconcile(
        Some(bytes),
        &resolved,
    );
    assert_eq!(
        output.expected_bytes, bytes,
        "Check-only assertions must preserve exact bytes when no value can be chosen."
    );
    assert_eq!(
        output.findings.len(),
        2,
        "Each unsatisfied check-only assertion must be reported."
    );

    let missing_output =
        <PackageJsonEngine as FileEngine<ResolvedPackageJsonRequirements>>::reconcile(
            None, &resolved,
        );
    assert_eq!(
        missing_output.expected_bytes, b"{}\n",
        "Check-only assertions must not invent values in a missing document."
    );
    assert_eq!(
        missing_output.findings.len(),
        2,
        "Missing check-only values must still be reported."
    );
}

#[test]
fn malformed_non_object_and_duplicate_json_suppress_field_findings() {
    for bytes in [
        b"{".as_slice(),
        b"[]".as_slice(),
        br#"{"packageManager":"pnpm@11","packageManager":"npm@10"}"#.as_slice(),
    ] {
        let output = reconcile_existing(bytes);
        assert_eq!(
            output.expected_bytes, bytes,
            "A parse failure must preserve supplied bytes."
        );
        assert_eq!(
            output.findings.len(),
            1,
            "A parse failure must suppress scalar findings."
        );
        assert!(
            matches!(output.findings.first(), Some(Finding::ParseError { .. })),
            "The finding must report parse or root shape."
        );
    }
}

#[test]
fn merge_agreement_exposes_all_immutable_resolved_getters() {
    let resolved = merge(vec![
        (provenance("policy-a"), complete_requirement("11.11.0")),
        (provenance("policy-b"), complete_requirement("11.11.0")),
    ]);
    assert_eq!(
        resolved
            .package_manager()
            .expect("packageManager must resolve.")
            .collected
            .len(),
        2,
        "Agreement must retain both contributors."
    );
    let nested = resolved.dev_engines_package_manager();
    assert!(
        nested.name().is_some(),
        "The nested name getter must expose resolved state."
    );
    assert!(
        nested.version().is_some(),
        "The nested version getter must expose resolved state."
    );
    assert!(
        nested.on_fail().is_some(),
        "The nested onFail getter must expose resolved state."
    );
}

#[test]
fn merge_reports_package_manager_and_nested_conflicts() {
    let error = PackageJsonRequirements::merge(vec![
        (provenance("policy-a"), complete_requirement("11.11.0")),
        (provenance("policy-b"), complete_requirement("11.12.0")),
    ])
    .expect_err("Different package manager versions must conflict.");
    assert_eq!(
        error.len(),
        2,
        "Top-level and nested versions must conflict independently."
    );
    assert_eq!(
        error.first().expect("The first conflict must exist.").key,
        "packageManager",
        "The first conflict must name packageManager."
    );
    assert_eq!(
        error.get(1).expect("The nested conflict must exist.").key,
        "devEngines.packageManager.version",
        "The nested conflict must name its file path."
    );
}

#[test]
fn merge_reports_name_and_on_fail_conflicts() {
    let mut other = complete_requirement("11.11.0");
    other.dev_engines_package_manager.name = Some(ScalarAssertion::Equals(
        "npm".to_owned(),
        "Require npm.".to_owned(),
    ));
    other.dev_engines_package_manager.on_fail = Some(ScalarAssertion::Equals(
        PackageManagerOnFail::Warn,
        "Warn on mismatch.".to_owned(),
    ));
    let conflicts = PackageJsonRequirements::merge(vec![
        (provenance("pnpm"), complete_requirement("11.11.0")),
        (provenance("npm"), other),
    ])
    .expect_err("Name and onFail disagreements must conflict.");
    assert_eq!(
        conflicts
            .iter()
            .map(|entry| entry.key.as_str())
            .collect::<Vec<_>>(),
        [
            "devEngines.packageManager.name",
            "devEngines.packageManager.onFail"
        ]
    );
}

#[test]
fn merge_and_reconcile_are_independent_of_policy_order() {
    let forward = merge(vec![
        (provenance("alpha"), complete_requirement("11.11.0")),
        (provenance("beta"), complete_requirement("11.11.0")),
    ]);
    let reverse = merge(vec![
        (provenance("beta"), complete_requirement("11.11.0")),
        (provenance("alpha"), complete_requirement("11.11.0")),
    ]);
    let forward_output = <PackageJsonEngine as FileEngine<_>>::reconcile(None, &forward);
    let reverse_output = <PackageJsonEngine as FileEngine<_>>::reconcile(None, &reverse);
    assert_eq!(forward_output.expected_bytes, reverse_output.expected_bytes);
    assert_eq!(
        format!("{:?}", forward_output.findings),
        format!("{:?}", reverse_output.findings)
    );
}

#[test]
fn erased_engine_dispatch_merges_requirements_and_reports_conflicts() {
    let engine = PackageJsonEngine;
    let requirements: Vec<(Provenance, Box<dyn EngineRequirement>)> = vec![
        (
            provenance("policy-a"),
            Box::new(complete_requirement("11.11.0")),
        ),
        (
            provenance("policy-b"),
            Box::new(complete_requirement("11.12.0")),
        ),
    ];
    let output = Engine::reconcile(&engine, None, &requirements);
    assert_eq!(
        output.findings.len(),
        2,
        "Erased dispatch must preserve both merge conflicts."
    );
    assert!(
        output
            .findings
            .iter()
            .all(|finding| matches!(finding, Finding::ConflictingRequirements { .. })),
        "Merge disagreement must stop reconciliation."
    );
}

#[test]
fn erased_engine_with_no_requirements_preserves_bytes_as_no_op() {
    let bytes = b"{ \"custom\": true }\n";
    let output = Engine::reconcile(&PackageJsonEngine, Some(bytes), &[]);
    assert_eq!(
        output.expected_bytes, bytes,
        "No requirements must preserve bytes exactly."
    );
    assert!(
        output.findings.is_empty(),
        "No requirements must produce no findings."
    );
}

#[test]
fn keyed_maps_create_entries_and_preserve_unmanaged_siblings() {
    let resolved = keyed_map_requirement();
    let bytes =
        br#"{"scripts":{"lint":"eslint ."},"devDependencies":{"eslint":"9.0.0"},"private":true}"#;
    let output = <PackageJsonEngine as FileEngine<_>>::reconcile(Some(bytes), &resolved);
    assert_eq!(
        output.findings.len(),
        2,
        "Each missing keyed scalar must be reported."
    );
    assert_eq!(
        output.expected_bytes,
        b"{\n  \"devDependencies\": {\n    \"eslint\": \"9.0.0\",\n    \"typescript\": \"7.0.2\"\n  },\n  \"private\": true,\n  \"scripts\": {\n    \"lint\": \"eslint .\",\n    \"typecheck\": \"tsc --build --noEmit tsconfig.json\"\n  }\n}\n",
        "Managed entries must compose with unrelated object members."
    );
    let selectors = output
        .findings
        .iter()
        .map(|finding| {
            let Finding::Mismatch { selector, .. } = finding else {
                panic!("Keyed scalar mismatches must use mismatch findings.");
            };
            selector.clone()
        })
        .collect::<Vec<_>>();
    assert_eq!(
        selectors,
        [Some("typecheck".to_owned()), Some("typescript".to_owned())],
        "Each keyed scalar must retain its map key as selector."
    );
}

#[test]
fn keyed_map_wrong_parent_shape_is_reported_without_replacement() {
    let resolved = keyed_map_requirement();
    let bytes = br#"{"scripts":[],"devDependencies":"wrong","private":true}"#;
    let output = <PackageJsonEngine as FileEngine<_>>::reconcile(Some(bytes), &resolved);
    assert_eq!(
        output.expected_bytes, bytes,
        "A present non-object parent must not be replaced."
    );
    assert_eq!(
        output.findings.len(),
        2,
        "Each invalid parent must report once."
    );
    assert!(
        output.findings.iter().all(|finding| matches!(
            finding,
            Finding::Mismatch { selector: None, attribution, .. }
                if attribution == &[provenance(POLICY)]
        )),
        "Parent shape findings must aggregate map attribution without a selector."
    );
}

#[test]
fn keyed_map_merge_retains_contributors_and_reports_conflicts_by_file_path() {
    let equal = merge(vec![
        (provenance("policy-a"), keyed_map_input("7.0.2")),
        (provenance("policy-b"), keyed_map_input("7.0.2")),
    ]);
    assert_eq!(
        equal
            .dev_dependencies()
            .get("typescript")
            .expect("typescript must resolve.")
            .collected
            .len(),
        2,
        "Equal keyed assertions must retain every contributor."
    );
    let conflicts = PackageJsonRequirements::merge(vec![
        (provenance("policy-a"), keyed_map_input("7.0.2")),
        (provenance("policy-b"), keyed_map_input("7.1.0")),
    ])
    .expect_err("Different keyed values must conflict.");
    assert_eq!(
        conflicts
            .iter()
            .map(|entry| entry.key.as_str())
            .collect::<Vec<_>>(),
        ["devDependencies.typescript"],
        "Map conflicts must identify their Package JSON path."
    );
}

#[test]
fn keyed_maps_apply_check_only_and_absent_scalar_algebra() {
    let requirement = PackageJsonRequirements {
        scripts: [
            (
                "present".to_owned(),
                ScalarAssertion::Present("Declare the script.".to_owned()),
            ),
            (
                "supported".to_owned(),
                ScalarAssertion::OneOf(
                    ["first".to_owned(), "second".to_owned()]
                        .into_iter()
                        .collect(),
                    "Use a supported script.".to_owned(),
                ),
            ),
            (
                "removed".to_owned(),
                ScalarAssertion::Absent("Remove the script.".to_owned()),
            ),
        ]
        .into_iter()
        .collect(),
        ..PackageJsonRequirements::default()
    };
    let resolved = merge(vec![(provenance(POLICY), requirement)]);
    let bytes = br#"{"scripts":{"removed":"old","kept":"unchanged"}}"#;
    let output = <PackageJsonEngine as FileEngine<_>>::reconcile(Some(bytes), &resolved);
    assert_eq!(output.findings.len(), 3);
    assert_eq!(
        output.expected_bytes, b"{\n  \"scripts\": {\n    \"kept\": \"unchanged\"\n  }\n}\n",
        "Check-only assertions must not invent values and absence must remove only its key."
    );
    assert_eq!(
        output
            .findings
            .iter()
            .map(|finding| {
                let Finding::Mismatch { selector, .. } = finding else {
                    panic!("Map scalar failures must be mismatch findings.");
                };
                selector.clone()
            })
            .collect::<Vec<_>>(),
        [
            Some("present".to_owned()),
            Some("removed".to_owned()),
            Some("supported".to_owned()),
        ]
    );
}

fn keyed_map_requirement() -> ResolvedPackageJsonRequirements {
    merge(vec![(provenance(POLICY), keyed_map_input("7.0.2"))])
}

fn keyed_map_input(version: &str) -> PackageJsonRequirements {
    PackageJsonRequirements {
        scripts: std::iter::once((
            "typecheck".to_owned(),
            ScalarAssertion::Equals(
                "tsc --build --noEmit tsconfig.json".to_owned(),
                "Require the typecheck script.".to_owned(),
            ),
        ))
        .collect(),
        dev_dependencies: std::iter::once((
            "typescript".to_owned(),
            ScalarAssertion::Equals(version.to_owned(), "Pin TypeScript.".to_owned()),
        ))
        .collect(),
        ..PackageJsonRequirements::default()
    }
}

fn resolved_complete() -> ResolvedPackageJsonRequirements {
    merge(vec![(provenance(POLICY), complete_requirement("11.11.0"))])
}

fn reconcile_existing(bytes: &[u8]) -> aqc_file_engine_core::EngineOutput {
    <PackageJsonEngine as FileEngine<ResolvedPackageJsonRequirements>>::reconcile(
        Some(bytes),
        &resolved_complete(),
    )
}

fn merge(
    requirements: Vec<(Provenance, PackageJsonRequirements)>,
) -> ResolvedPackageJsonRequirements {
    PackageJsonRequirements::merge(requirements).expect("Compatible requirements must merge.")
}

fn complete_requirement(version: &str) -> PackageJsonRequirements {
    PackageJsonRequirements {
        package_manager: Some(ScalarAssertion::Equals(
            format!("pnpm@{version}"),
            "Pin the package manager.".to_owned(),
        )),
        dev_engines_package_manager: DevEnginePackageManagerRequirements {
            name: Some(ScalarAssertion::Equals(
                "pnpm".to_owned(),
                "Require pnpm.".to_owned(),
            )),
            version: Some(ScalarAssertion::Equals(
                version.to_owned(),
                "Pin the pnpm version.".to_owned(),
            )),
            on_fail: Some(ScalarAssertion::Equals(
                PackageManagerOnFail::Error,
                "Fail on a package manager mismatch.".to_owned(),
            )),
        },
        ..PackageJsonRequirements::default()
    }
}

fn provenance(policy: &str) -> Provenance {
    Provenance {
        policy: policy.to_owned(),
    }
}
