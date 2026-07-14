use std::path::Path;

use aqc_file_engine_core::{
    FileEngine, Finding, ForbiddenGlobRequirements, Provenance, ScalarAssertion,
};
use aqc_package_json_engine::{
    DevEnginePackageManagerRequirements, PackageJsonEngine, PackageJsonRequirements,
    PackageManagerOnFail,
};
use aqc_pnpm_workspace_yaml_engine::{
    PnpmPackageSelectorGlob, PnpmWorkspaceYamlEngine, PnpmWorkspaceYamlRequirements,
};
use aqc_yaml_engine_core::{YamlFieldValue, parse_yaml_mapping};
use serde_json::{Value, json};

fn provenance() -> Provenance {
    Provenance {
        policy: "consumer-policy".to_owned(),
    }
}

fn finding_summary(finding: &Finding) -> Value {
    match finding {
        Finding::Mismatch {
            key,
            selector,
            attribution,
            ..
        } => json!({
            "kind": "mismatch",
            "key": key,
            "selector": selector,
            "policies": attribution.iter().map(|item| item.policy.as_str()).collect::<Vec<_>>(),
        }),
        Finding::ParseError { message, .. } => json!({"kind": "parse", "message": message}),
        Finding::InvalidRequirements { key, .. } => json!({"kind": "invalid", "key": key}),
        Finding::ConflictingRequirements { key, .. } => json!({"kind": "conflict", "key": key}),
        Finding::UnwritableRequiredKey { key, .. } => json!({"kind": "unwritable", "key": key}),
        Finding::InternalError { message } => json!({"kind": "internal", "message": message}),
    }
}

fn main() {
    let fixture = std::env::args().nth(1).expect("fixture path");
    let fixture: Value =
        serde_json::from_slice(&std::fs::read(Path::new(&fixture)).expect("fixture bytes"))
            .expect("fixture JSON");
    let required = [
        "duplicate-json",
        "missing-package-json",
        "merged-yaml",
        "quoted-merge-key",
        "forbidden-selector",
        "exact-only-isolation",
    ];
    for name in required {
        assert!(
            fixture["cases"]
                .as_array()
                .is_some_and(|cases| cases.iter().any(|item| item == name))
        );
    }

    let (_, duplicate_findings) = aqc_json_engine_core::parse_object_or_report(
        Some(br#"{"setting":true,"setting":false}"#),
        "config.json",
    );

    let package = PackageJsonRequirements {
        package_manager: Some(ScalarAssertion::Equals(
            "tool@2.3.4".to_owned(),
            "pin tool".to_owned(),
        )),
        dev_engines_package_manager: DevEnginePackageManagerRequirements {
            name: Some(ScalarAssertion::Equals(
                "tool".to_owned(),
                "name tool".to_owned(),
            )),
            version: Some(ScalarAssertion::Equals(
                "2.3.4".to_owned(),
                "pin version".to_owned(),
            )),
            on_fail: Some(ScalarAssertion::Equals(
                PackageManagerOnFail::Error,
                "fail mismatch".to_owned(),
            )),
        },
        ..PackageJsonRequirements::default()
    };
    let package =
        PackageJsonRequirements::merge(vec![(provenance(), package)]).expect("package merge");
    let package_output = <PackageJsonEngine as FileEngine<_>>::reconcile(None, &package);
    let package_fixed_point = <PackageJsonEngine as FileEngine<_>>::reconcile(
        Some(&package_output.expected_bytes),
        &package,
    );
    assert!(package_fixed_point.findings.is_empty());
    assert_eq!(
        package_fixed_point.expected_bytes,
        package_output.expected_bytes
    );

    let merged = parse_yaml_mapping(
        Some(b"defaults: &defaults\n  inherited: true\n<<: *defaults\ndirect: false\n"),
        "config.yaml",
    )
    .expect("merged YAML");
    let quoted =
        parse_yaml_mapping(Some(b"\"<<\": ordinary\n"), "config.yaml").expect("quoted merge key");
    let quoted_value = match quoted.field("<<").expect("quoted value") {
        Some(YamlFieldValue::String(value)) => value,
        value => panic!("unexpected quoted merge-key value: {value:?}"),
    };

    let forbidden = PnpmWorkspaceYamlRequirements {
        forbidden_trust_policy_exclude_globs: ForbiddenGlobRequirements {
            globs: vec![(
                PnpmPackageSelectorGlob {
                    glob: "@scope/*".to_owned(),
                },
                "forbid selector".to_owned(),
            )],
        },
        ..PnpmWorkspaceYamlRequirements::default()
    };
    let forbidden = PnpmWorkspaceYamlRequirements::merge(vec![(provenance(), forbidden)])
        .expect("forbidden selector merge");
    let forbidden_output = <PnpmWorkspaceYamlEngine as FileEngine<_>>::reconcile(
        Some(b"trustPolicyExclude: ['@scope/unsafe', safe]\n"),
        &forbidden,
    );
    parse_yaml_mapping(
        Some(&forbidden_output.expected_bytes),
        "pnpm-workspace.yaml",
    )
    .expect("generated YAML");
    let forbidden_fixed_point = <PnpmWorkspaceYamlEngine as FileEngine<_>>::reconcile(
        Some(&forbidden_output.expected_bytes),
        &forbidden,
    );
    assert!(forbidden_fixed_point.findings.is_empty());
    assert_eq!(
        forbidden_fixed_point.expected_bytes,
        forbidden_output.expected_bytes
    );

    let exact = PnpmWorkspaceYamlRequirements {
        exact_settings: Some("only represented settings".to_owned()),
        ..PnpmWorkspaceYamlRequirements::default()
    };
    let exact =
        PnpmWorkspaceYamlRequirements::merge(vec![(provenance(), exact)]).expect("exact merge");
    let exact_output = <PnpmWorkspaceYamlEngine as FileEngine<_>>::reconcile(
        Some(b"allowBuilds: [tool]\n"),
        &exact,
    );

    let output = json!({
        "duplicateJson": duplicate_findings.iter().map(finding_summary).collect::<Vec<_>>(),
        "missingPackageJson": {
            "expected": String::from_utf8(package_output.expected_bytes).expect("JSON UTF-8"),
            "findings": package_output.findings.iter().map(finding_summary).collect::<Vec<_>>(),
        },
        "mergedYaml": {
            "keys": merged.effective_keys().expect("effective keys"),
            "inherited": matches!(merged.field("inherited"), Ok(Some(YamlFieldValue::Boolean(true)))),
            "direct": matches!(merged.field("direct"), Ok(Some(YamlFieldValue::Boolean(false)))),
        },
        "quotedMergeKey": {
            "keys": quoted.effective_keys().expect("quoted key"),
            "value": quoted_value,
        },
        "forbiddenSelector": {
            "expected": String::from_utf8(forbidden_output.expected_bytes).expect("YAML UTF-8"),
            "findings": forbidden_output.findings.iter().map(finding_summary).collect::<Vec<_>>(),
        },
        "exactOnlyIsolation": {
            "findingCount": exact_output.findings.len(),
            "findings": exact_output.findings.iter().map(finding_summary).collect::<Vec<_>>(),
        },
    });
    println!("{}", serde_json::to_string(&output).expect("output JSON"));
}
