use std::collections::BTreeSet;
use std::path::Path;

use aqc_file_engine_core::{Engine, EngineRequirement, FileEngine, Finding, Provenance};
use aqc_json_engine_core::{JsonParseOptions, parse_object_or_report};
use aqc_package_json_engine::{PackageJsonEngine, PackageJsonRequirements};
use aqc_tsconfig_json_engine::{
    ScalarAssertion, TsconfigBooleanCompilerOption, TsconfigJsonEngine, TsconfigJsonRequirements,
};
use serde_json::{Value, json};

const OPTIONS: [(TsconfigBooleanCompilerOption, &str, bool); 22] = [
    (TsconfigBooleanCompilerOption::Strict, "strict", true),
    (
        TsconfigBooleanCompilerOption::AlwaysStrict,
        "alwaysStrict",
        true,
    ),
    (
        TsconfigBooleanCompilerOption::NoImplicitAny,
        "noImplicitAny",
        true,
    ),
    (
        TsconfigBooleanCompilerOption::NoImplicitThis,
        "noImplicitThis",
        true,
    ),
    (
        TsconfigBooleanCompilerOption::StrictBindCallApply,
        "strictBindCallApply",
        true,
    ),
    (
        TsconfigBooleanCompilerOption::StrictFunctionTypes,
        "strictFunctionTypes",
        true,
    ),
    (
        TsconfigBooleanCompilerOption::StrictNullChecks,
        "strictNullChecks",
        true,
    ),
    (
        TsconfigBooleanCompilerOption::StrictPropertyInitialization,
        "strictPropertyInitialization",
        true,
    ),
    (
        TsconfigBooleanCompilerOption::UseUnknownInCatchVariables,
        "useUnknownInCatchVariables",
        true,
    ),
    (
        TsconfigBooleanCompilerOption::StrictBuiltinIteratorReturn,
        "strictBuiltinIteratorReturn",
        true,
    ),
    (
        TsconfigBooleanCompilerOption::NoImplicitReturns,
        "noImplicitReturns",
        true,
    ),
    (
        TsconfigBooleanCompilerOption::NoUnusedLocals,
        "noUnusedLocals",
        true,
    ),
    (
        TsconfigBooleanCompilerOption::NoUnusedParameters,
        "noUnusedParameters",
        true,
    ),
    (
        TsconfigBooleanCompilerOption::NoUncheckedIndexedAccess,
        "noUncheckedIndexedAccess",
        true,
    ),
    (
        TsconfigBooleanCompilerOption::ExactOptionalPropertyTypes,
        "exactOptionalPropertyTypes",
        true,
    ),
    (
        TsconfigBooleanCompilerOption::NoPropertyAccessFromIndexSignature,
        "noPropertyAccessFromIndexSignature",
        true,
    ),
    (
        TsconfigBooleanCompilerOption::NoImplicitOverride,
        "noImplicitOverride",
        true,
    ),
    (
        TsconfigBooleanCompilerOption::NoFallthroughCasesInSwitch,
        "noFallthroughCasesInSwitch",
        true,
    ),
    (
        TsconfigBooleanCompilerOption::ForceConsistentCasingInFileNames,
        "forceConsistentCasingInFileNames",
        true,
    ),
    (
        TsconfigBooleanCompilerOption::AllowUnreachableCode,
        "allowUnreachableCode",
        false,
    ),
    (
        TsconfigBooleanCompilerOption::AllowUnusedLabels,
        "allowUnusedLabels",
        false,
    ),
    (TsconfigBooleanCompilerOption::NoCheck, "noCheck", false),
];

fn provenance(policy: &str) -> Provenance {
    Provenance {
        policy: policy.to_owned(),
    }
}

fn finding(finding: &Finding) -> Value {
    match finding {
        Finding::Mismatch {
            key,
            selector,
            current,
            expected,
            attribution,
            ..
        } => json!({
            "kind": "mismatch", "key": key, "selector": selector, "current": current,
            "expected": expected,
            "policies": attribution.iter().map(|item| item.policy.as_str()).collect::<Vec<_>>(),
        }),
        Finding::ParseError { message, .. } => json!({"kind": "parse", "message": message}),
        Finding::ConflictingRequirements {
            key,
            contributors,
            reason,
        } => json!({
            "kind": "conflict", "key": key, "reason": reason, "contributors": contributors,
        }),
        Finding::InvalidRequirements {
            key, contributors, ..
        } => {
            json!({"kind": "invalid", "key": key, "contributors": contributors})
        }
        Finding::UnwritableRequiredKey {
            key, attribution, ..
        } => json!({
            "kind": "unwritable", "key": key,
            "policies": attribution.iter().map(|item| item.policy.as_str()).collect::<Vec<_>>(),
        }),
        Finding::InternalError { message } => json!({"kind": "internal", "message": message}),
    }
}

fn findings(items: &[Finding]) -> Vec<Value> {
    items.iter().map(finding).collect()
}

fn one_option(option: TsconfigBooleanCompilerOption, required: bool) -> TsconfigJsonRequirements {
    TsconfigJsonRequirements {
        boolean_compiler_options: [(
            option,
            ScalarAssertion::Equals(required, format!("Require {}.", option.file_key())),
        )]
        .into_iter()
        .collect(),
    }
}

fn baseline() -> TsconfigJsonRequirements {
    TsconfigJsonRequirements {
        boolean_compiler_options: OPTIONS
            .into_iter()
            .map(|(option, key, required)| {
                (
                    option,
                    ScalarAssertion::Equals(required, format!("Require {key}.")),
                )
            })
            .collect(),
    }
}

fn resolve_ts(
    requirement: TsconfigJsonRequirements,
) -> aqc_tsconfig_json_engine::ResolvedTsconfigJsonRequirements {
    TsconfigJsonRequirements::merge(vec![(provenance("tsc-policy"), requirement)])
        .expect("TSConfig requirement must merge")
}

fn package_requirement() -> PackageJsonRequirements {
    PackageJsonRequirements {
        scripts: [(
            "typecheck".to_owned(),
            ScalarAssertion::Equals(
                "tsc --build tsconfig.json --noCheck false".to_owned(),
                "Require typecheck script.".to_owned(),
            ),
        )]
        .into_iter()
        .collect(),
        dev_dependencies: [(
            "typescript".to_owned(),
            ScalarAssertion::Equals("7.0.2".to_owned(), "Require TypeScript.".to_owned()),
        )]
        .into_iter()
        .collect(),
        ..PackageJsonRequirements::default()
    }
}

fn resolve_package(
    requirement: PackageJsonRequirements,
) -> aqc_package_json_engine::ResolvedPackageJsonRequirements {
    PackageJsonRequirements::merge(vec![(provenance("tsc-policy"), requirement)])
        .expect("Package JSON requirement must merge")
}

fn jsonc_dialect() -> Value {
    let accepted = [
        (
            "comments",
            b"{ // comment\n \"compilerOptions\": {} }\n".as_slice(),
        ),
        (
            "trailing-commas",
            b"{\"compilerOptions\": {\"strict\": true,},}\n".as_slice(),
        ),
        (
            "hexadecimal",
            b"{\"unmanaged\": 0x10, \"compilerOptions\": {}}\n".as_slice(),
        ),
        (
            "javascript-numbers",
            b"{\"binary\":0b10,\"octal\":0o10,\"leading\":.5,\"trailing\":1.,\"separator\":1_000,\"compilerOptions\":{}}\n".as_slice(),
        ),
        (
            "utf8-bom",
            b"\xef\xbb\xbf{\"compilerOptions\":{}}\n".as_slice(),
        ),
    ];
    let rejected = [
        ("single-quotes", b"{'compilerOptions': {}}".as_slice()),
        ("loose-property-names", b"{compilerOptions: {}}".as_slice()),
        ("omitted-commas", b"{\"first\": 1 \"second\": 2}".as_slice()),
        ("unary-plus", b"{\"unmanaged\": +1}".as_slice()),
        ("bigint", b"{\"unmanaged\": 1n}".as_slice()),
        ("legacy-octal", b"{\"unmanaged\": 010}".as_slice()),
    ];
    let empty = resolve_ts(TsconfigJsonRequirements::default());
    json!({
        "accepted": accepted.into_iter().map(|(name, bytes)| {
            let output = <TsconfigJsonEngine as FileEngine<_>>::reconcile(Some(bytes), &empty);
            assert!(output.findings.is_empty(), "accepted JSONC dialect: {name}");
            assert_eq!(output.expected_bytes, bytes, "accepted JSONC exact preservation: {name}");
            json!({"case": name, "accepted": true, "exactBytes": true})
        }).collect::<Vec<_>>(),
        "rejected": rejected.into_iter().map(|(name, bytes)| {
            let output = <TsconfigJsonEngine as FileEngine<_>>::reconcile(Some(bytes), &empty);
            assert_eq!(output.expected_bytes, bytes, "rejected JSONC preservation: {name}");
            assert!(matches!(output.findings.as_slice(), [Finding::ParseError { .. }]));
            json!({"case": name, "accepted": false, "bytesPreserved": true, "finding": finding(&output.findings[0])})
        }).collect::<Vec<_>>(),
    })
}

fn jsonc_preservation() -> Value {
    let valid = b"\xef\xbb\xbf{\n  // root comment\n  \"extends\": \"./base.json\",\n  \"binary\": 0b10,\n  \"separated\": 1_000,\n  \"compilerOptions\": {\n    // managed comment\n    \"strict\": true,\n    \"module\": \"preserve\",\n  },\n  \"references\": [{\"path\": \"./child\"}],\n}\n";
    let unchanged = <TsconfigJsonEngine as FileEngine<_>>::reconcile(
        Some(valid),
        &resolve_ts(one_option(TsconfigBooleanCompilerOption::Strict, true)),
    );
    assert!(unchanged.findings.is_empty());
    assert_eq!(unchanged.expected_bytes, valid);

    let changed_input = String::from_utf8(valid.to_vec())
        .expect("fixture UTF-8")
        .replace("\"strict\": true", "\"strict\": false");
    let changed = <TsconfigJsonEngine as FileEngine<_>>::reconcile(
        Some(changed_input.as_bytes()),
        &resolve_ts(one_option(TsconfigBooleanCompilerOption::Strict, true)),
    );
    let rendered = String::from_utf8(changed.expected_bytes.clone()).expect("JSONC output UTF-8");
    for fragment in [
        "// root comment",
        "// managed comment",
        "\"binary\": 0b10,",
        "\"separated\": 1_000,",
        "\"module\": \"preserve\",",
        "\"extends\"",
        "\"references\"",
    ] {
        assert!(
            rendered.contains(fragment),
            "changed output must preserve {fragment}"
        );
    }
    assert!(changed.expected_bytes.starts_with(b"\xef\xbb\xbf"));
    assert!(rendered.contains("\"strict\": true,"));
    let fixed = <TsconfigJsonEngine as FileEngine<_>>::reconcile(
        Some(&changed.expected_bytes),
        &resolve_ts(one_option(TsconfigBooleanCompilerOption::Strict, true)),
    );
    assert!(fixed.findings.is_empty());
    assert_eq!(fixed.expected_bytes, changed.expected_bytes);

    let insertion_input = String::from_utf8(valid.to_vec())
        .expect("fixture UTF-8")
        .replace("    \"strict\": true,\n", "");
    let inserted = <TsconfigJsonEngine as FileEngine<_>>::reconcile(
        Some(insertion_input.as_bytes()),
        &resolve_ts(one_option(TsconfigBooleanCompilerOption::Strict, true)),
    );
    let inserted_rendered =
        String::from_utf8(inserted.expected_bytes.clone()).expect("inserted JSONC output UTF-8");
    for fragment in [
        "// root comment",
        "// managed comment",
        "\"binary\": 0b10,",
        "\"separated\": 1_000,",
        "\"module\": \"preserve\",",
        "\"extends\"",
        "\"references\"",
    ] {
        assert!(
            inserted_rendered.contains(fragment),
            "inserted output must preserve {fragment}"
        );
    }
    assert!(inserted.expected_bytes.starts_with(b"\xef\xbb\xbf"));
    assert!(inserted_rendered.contains("\"strict\": true"));
    json!({
        "unchanged": {"exactBytes": true, "findings": findings(&unchanged.findings)},
        "changed": {"expected": rendered, "findings": findings(&changed.findings), "stableRerun": true, "bomPreserved": true, "extendedNumbersPreserved": true},
        "inserted": {"expected": inserted_rendered, "findings": findings(&inserted.findings), "insertionPreserved": true, "bomPreserved": true, "extendedNumbersPreserved": true},
    })
}

fn jsonc_failures() -> Value {
    let resolved = resolve_ts(baseline());
    let failures: [(&str, &[u8]); 11] = [
        ("empty-bytes", b""),
        ("invalid-utf8", &[0xff, 0xfe]),
        ("malformed", b"{"),
        ("non-object-array", b"[]"),
        ("non-object-null", b"null"),
        ("non-object-string", b"\"value\""),
        ("non-object-boolean", b"true"),
        ("non-object-number", b"1"),
        ("duplicate-root", b"{\"x\":1,\"x\":2}"),
        (
            "duplicate-nested",
            b"{\"compilerOptions\":{\"strict\":true,\"strict\":false}}",
        ),
        ("duplicate-in-array", b"{\"items\":[{\"x\":1,\"x\":2}]}"),
    ];
    let parse = failures.into_iter().map(|(name, bytes)| {
        let output = <TsconfigJsonEngine as FileEngine<_>>::reconcile(Some(bytes), &resolved);
        assert_eq!(output.expected_bytes, bytes, "failure must preserve bytes: {name}");
        assert!(matches!(output.findings.as_slice(), [Finding::ParseError { .. }]));
        json!({"case": name, "bytesPreserved": true, "suppressedOptionFindings": true, "finding": finding(&output.findings[0])})
    }).collect::<Vec<_>>();
    let shapes: [(&str, &[u8]); 5] = [
        ("null", b"{\"compilerOptions\":null}"),
        ("array", b"{\"compilerOptions\":[]}"),
        ("string", b"{\"compilerOptions\":\"wrong\"}"),
        ("boolean", b"{\"compilerOptions\":true}"),
        ("number", b"{\"compilerOptions\":1}"),
    ];
    let shape = shapes.into_iter().map(|(name, bytes)| {
        let output = <TsconfigJsonEngine as FileEngine<_>>::reconcile(Some(bytes), &resolved);
        assert_eq!(output.expected_bytes, bytes);
        assert!(matches!(output.findings.as_slice(), [Finding::Mismatch { key, selector: None, .. }] if key == "compilerOptions"));
        json!({"case": name, "bytesPreserved": true, "suppressedOptionFindings": true, "finding": finding(&output.findings[0])})
    }).collect::<Vec<_>>();
    json!({"parseAndRootFailures": parse, "compilerOptionsShapeFailures": shape})
}

fn compiler_options(contract: &Value) -> Value {
    let declared = contract["compilerOptions"]
        .as_array()
        .expect("compiler options contract");
    assert_eq!(declared.len(), OPTIONS.len());
    let declared_keys = declared
        .iter()
        .map(|item| item["key"].as_str().expect("option key"))
        .collect::<BTreeSet<_>>();
    let api_keys = OPTIONS
        .into_iter()
        .map(|(option, key, _)| {
            assert_eq!(option.file_key(), key);
            key
        })
        .collect::<BTreeSet<_>>();
    assert_eq!(declared_keys, api_keys);

    let cases = OPTIONS.into_iter().map(|(option, key, required)| {
        let resolved = resolve_ts(one_option(option, required));
        let inputs = [
            ("missing", "{\"compilerOptions\":{}}".to_owned()),
            ("reversed", format!("{{\"compilerOptions\":{{\"{key}\":{}}}}}", !required)),
            ("wrong-type", format!("{{\"compilerOptions\":{{\"{key}\":\"wrong\"}}}}")),
        ];
        let states = inputs.into_iter().map(|(state, input)| {
            let output = <TsconfigJsonEngine as FileEngine<_>>::reconcile(Some(input.as_bytes()), &resolved);
            assert!(matches!(output.findings.as_slice(), [Finding::Mismatch { key: finding_key, selector: Some(selector), .. }]
                if finding_key == &format!("compilerOptions.{key}") && selector == key));
            let fixed = <TsconfigJsonEngine as FileEngine<_>>::reconcile(Some(&output.expected_bytes), &resolved);
            assert!(fixed.findings.is_empty(), "fixed point for {key} {state}");
            assert_eq!(fixed.expected_bytes, output.expected_bytes);
            json!({"state": state, "finding": finding(&output.findings[0]), "corrected": true, "stableRerun": true})
        }).collect::<Vec<_>>();
        json!({"key": key, "required": required, "states": states})
    }).collect::<Vec<_>>();
    json!({"count": cases.len(), "cases": cases})
}

fn package_json_maps() -> Value {
    let resolved = resolve_package(package_requirement());
    let inputs = [
        ("missing", b"{}".as_slice()),
        ("different", b"{\"scripts\":{\"typecheck\":\"tsc -p other.json\"},\"devDependencies\":{\"typescript\":\"next\"}}".as_slice()),
        ("wrong-type-boolean", b"{\"scripts\":{\"typecheck\":true},\"devDependencies\":{\"typescript\":false}}".as_slice()),
        ("wrong-type-number", b"{\"scripts\":{\"typecheck\":1},\"devDependencies\":{\"typescript\":2}}".as_slice()),
        ("wrong-type-null", b"{\"scripts\":{\"typecheck\":null},\"devDependencies\":{\"typescript\":null}}".as_slice()),
        ("wrong-type-array", b"{\"scripts\":{\"typecheck\":[]},\"devDependencies\":{\"typescript\":[]}}".as_slice()),
        ("wrong-type-object", b"{\"scripts\":{\"typecheck\":{}},\"devDependencies\":{\"typescript\":{}}}".as_slice()),
        ("wrong-section-and-siblings", b"{\"scripts\":{\"lint\":\"eslint .\"},\"dependencies\":{\"typescript\":\"7.0.2\"},\"devDependencies\":{\"eslint\":\"9.0.0\"},\"private\":true}".as_slice()),
    ];
    let leaves = inputs
        .into_iter()
        .map(|(name, bytes)| {
            let output = <PackageJsonEngine as FileEngine<_>>::reconcile(Some(bytes), &resolved);
            assert_eq!(output.findings.len(), 2);
            assert_eq!(
                output
                    .findings
                    .iter()
                    .filter_map(|item| match item {
                        Finding::Mismatch { selector, .. } => selector.as_deref(),
                        _ => None,
                    })
                    .collect::<BTreeSet<_>>(),
                BTreeSet::from(["typecheck", "typescript"])
            );
            let fixed = <PackageJsonEngine as FileEngine<_>>::reconcile(
                Some(&output.expected_bytes),
                &resolved,
            );
            assert!(fixed.findings.is_empty());
            let expected = String::from_utf8(output.expected_bytes).expect("Package JSON UTF-8");
            if name == "wrong-section-and-siblings" {
                let parsed: Value = serde_json::from_str(&expected).expect("Package JSON object");
                assert_eq!(parsed["dependencies"]["typescript"], "7.0.2");
                assert_eq!(parsed["scripts"]["lint"], "eslint .");
                assert_eq!(parsed["devDependencies"]["eslint"], "9.0.0");
                assert_eq!(parsed["private"], true);
            }
            json!({"case": name, "expected": expected, "findings": findings(&output.findings), "stableRerun": true})
        })
        .collect::<Vec<_>>();

    let parent_inputs = [
        (
            "null",
            b"{\"scripts\":null,\"devDependencies\":null,\"private\":true}".as_slice(),
        ),
        (
            "array",
            b"{\"scripts\":[],\"devDependencies\":[],\"private\":true}".as_slice(),
        ),
        (
            "string",
            b"{\"scripts\":\"wrong\",\"devDependencies\":\"wrong\",\"private\":true}".as_slice(),
        ),
        (
            "boolean",
            b"{\"scripts\":true,\"devDependencies\":false,\"private\":true}".as_slice(),
        ),
        (
            "number",
            b"{\"scripts\":1,\"devDependencies\":2,\"private\":true}".as_slice(),
        ),
    ];
    let parents = parent_inputs
        .into_iter()
        .map(|(name, bytes)| {
            let output = <PackageJsonEngine as FileEngine<_>>::reconcile(Some(bytes), &resolved);
            assert_eq!(output.expected_bytes, bytes);
            assert_eq!(output.findings.len(), 2);
            assert!(
                output
                    .findings
                    .iter()
                    .all(|item| matches!(item, Finding::Mismatch { selector: None, .. }))
            );
            json!({"case": name, "bytesPreserved": true, "findings": findings(&output.findings)})
        })
        .collect::<Vec<_>>();

    let algebra = PackageJsonRequirements {
        scripts: [
            (
                "present".to_owned(),
                ScalarAssertion::Present("Require any value.".to_owned()),
            ),
            (
                "oneOf".to_owned(),
                ScalarAssertion::OneOf(
                    BTreeSet::from(["a".to_owned(), "b".to_owned()]),
                    "Require supported value.".to_owned(),
                ),
            ),
            (
                "remove".to_owned(),
                ScalarAssertion::Absent("Remove value.".to_owned()),
            ),
        ]
        .into_iter()
        .collect(),
        ..PackageJsonRequirements::default()
    };
    let algebra = resolve_package(algebra);
    let algebra_output = <PackageJsonEngine as FileEngine<_>>::reconcile(
        Some(b"{\"scripts\":{\"remove\":\"old\",\"kept\":\"yes\"}}"),
        &algebra,
    );
    assert_eq!(algebra_output.findings.len(), 3);
    let algebra_expected =
        String::from_utf8(algebra_output.expected_bytes).expect("Package JSON UTF-8");
    let parsed: Value = serde_json::from_str(&algebra_expected).expect("Package JSON object");
    assert_eq!(parsed["scripts"]["kept"], "yes");
    assert!(parsed["scripts"].get("remove").is_none());
    assert!(parsed["scripts"].get("present").is_none());
    assert!(parsed["scripts"].get("oneOf").is_none());
    json!({
        "leafCases": leaves,
        "parentShapeCases": parents,
        "scalarAlgebra": {"expected": algebra_expected, "findings": findings(&algebra_output.findings)},
    })
}

fn merge_and_attribution() -> Value {
    let equal_ts = TsconfigJsonRequirements::merge(vec![
        (
            provenance("alpha"),
            one_option(TsconfigBooleanCompilerOption::Strict, true),
        ),
        (
            provenance("beta"),
            one_option(TsconfigBooleanCompilerOption::Strict, true),
        ),
    ])
    .expect("equal TS requirements");
    let equal_output = <TsconfigJsonEngine as FileEngine<_>>::reconcile(
        Some(b"{\"compilerOptions\":{\"strict\":false}}"),
        &equal_ts,
    );
    assert!(
        matches!(equal_output.findings.as_slice(), [Finding::Mismatch { attribution, .. }] if attribution == &[provenance("alpha"), provenance("beta")])
    );

    let ts_requirements: Vec<(Provenance, Box<dyn EngineRequirement>)> = vec![
        (
            provenance("alpha"),
            Box::new(one_option(TsconfigBooleanCompilerOption::Strict, true)),
        ),
        (
            provenance("beta"),
            Box::new(one_option(TsconfigBooleanCompilerOption::Strict, false)),
        ),
    ];
    let sentinel = b"not parsed because merge fails";
    let ts_conflict = Engine::reconcile(&TsconfigJsonEngine, Some(sentinel), &ts_requirements);
    assert_eq!(ts_conflict.expected_bytes, sentinel);
    assert!(
        matches!(ts_conflict.findings.as_slice(), [Finding::ConflictingRequirements { key, .. }] if key == "compilerOptions.strict")
    );

    let package_a = package_requirement();
    let mut package_b = package_requirement();
    package_b.scripts.insert(
        "typecheck".to_owned(),
        ScalarAssertion::Equals("other".to_owned(), "Other script.".to_owned()),
    );
    package_b.dev_dependencies.insert(
        "typescript".to_owned(),
        ScalarAssertion::Equals("7.1.0".to_owned(), "Other version.".to_owned()),
    );
    let package_requirements: Vec<(Provenance, Box<dyn EngineRequirement>)> = vec![
        (provenance("alpha"), Box::new(package_a.clone())),
        (provenance("beta"), Box::new(package_b)),
    ];
    let package_conflict =
        Engine::reconcile(&PackageJsonEngine, Some(sentinel), &package_requirements);
    assert_eq!(package_conflict.expected_bytes, sentinel);
    assert_eq!(package_conflict.findings.len(), 2);

    let equal_package = PackageJsonRequirements::merge(vec![
        (provenance("alpha"), package_a.clone()),
        (provenance("beta"), package_a),
    ])
    .expect("equal package requirements");
    assert_eq!(
        equal_package.scripts()["typecheck"].attribution(),
        vec![provenance("alpha"), provenance("beta")]
    );
    assert_eq!(
        equal_package.dev_dependencies()["typescript"].attribution(),
        vec![provenance("alpha"), provenance("beta")]
    );

    let ordered = TsconfigJsonRequirements {
        boolean_compiler_options: [(
            TsconfigBooleanCompilerOption::Strict,
            ScalarAssertion::AtLeast(true, "Unsupported boolean ordering.".to_owned()),
        )]
        .into_iter()
        .collect(),
    };
    let ordered_conflict = TsconfigJsonRequirements::merge(vec![(provenance("alpha"), ordered)])
        .expect_err("boolean ordering must conflict");
    assert_eq!(ordered_conflict.len(), 1);
    assert_eq!(ordered_conflict[0].reason, "scalar-order-unsupported");
    json!({
        "equalTsAttribution": findings(&equal_output.findings),
        "tsBooleanConflict": {"bytesPreserved": true, "findings": findings(&ts_conflict.findings)},
        "packageMapConflicts": {"bytesPreserved": true, "findings": findings(&package_conflict.findings)},
        "equalPackageAttribution": {
            "scripts.typecheck": equal_package.scripts()["typecheck"].attribution().into_iter().map(|item| item.policy).collect::<Vec<_>>(),
            "devDependencies.typescript": equal_package.dev_dependencies()["typescript"].attribution().into_iter().map(|item| item.policy).collect::<Vec<_>>(),
        },
        "orderedBooleanConflict": {
            "key": ordered_conflict[0].key,
            "reason": ordered_conflict[0].reason,
            "contributors": ordered_conflict[0].contributors.iter().map(|(item, value)| json!([item.policy, value])).collect::<Vec<_>>(),
        },
    })
}

fn absent_file_init() -> Value {
    let ts = resolve_ts(baseline());
    let ts_output = <TsconfigJsonEngine as FileEngine<_>>::reconcile(None, &ts);
    assert_eq!(ts_output.findings.len(), 22);
    let ts_fixed =
        <TsconfigJsonEngine as FileEngine<_>>::reconcile(Some(&ts_output.expected_bytes), &ts);
    assert!(ts_fixed.findings.is_empty());
    assert_eq!(ts_fixed.expected_bytes, ts_output.expected_bytes);

    let package = resolve_package(package_requirement());
    let package_output = <PackageJsonEngine as FileEngine<_>>::reconcile(None, &package);
    assert_eq!(package_output.findings.len(), 2);
    let package_fixed = <PackageJsonEngine as FileEngine<_>>::reconcile(
        Some(&package_output.expected_bytes),
        &package,
    );
    assert!(package_fixed.findings.is_empty());
    assert_eq!(package_fixed.expected_bytes, package_output.expected_bytes);
    json!({
        "tsconfig": {"expected": String::from_utf8(ts_output.expected_bytes).expect("TSConfig UTF-8"), "findingCount": 22, "stableRerun": true},
        "packageJson": {"expected": String::from_utf8(package_output.expected_bytes).expect("Package JSON UTF-8"), "findingCount": 2, "stableRerun": true},
    })
}

fn public_api() -> Value {
    let manifest = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml"));
    assert!(
        !manifest.contains("path ="),
        "probe dependencies must use registry coordinates"
    );
    let (parsed, parse_findings) =
        parse_object_or_report(Some(b"{}"), "consumer.jsonc", jsonc_options());
    assert!(parsed.is_some());
    assert!(parse_findings.is_empty());
    let ts = TsconfigJsonRequirements::merge(vec![(provenance("consumer"), baseline())])
        .expect("public TS merge");
    let package =
        PackageJsonRequirements::merge(vec![(provenance("consumer"), package_requirement())])
            .expect("public package merge");
    assert_eq!(ts.boolean_compiler_options().len(), 22);
    assert_eq!(package.scripts().len(), 1);
    assert_eq!(package.dev_dependencies().len(), 1);
    json!({
        "registryOnlyDependencies": true,
        "jsoncFacadeParsed": true,
        "tsconfigEngineId": aqc_tsconfig_json_engine::ENGINE_ID,
        "packageJsonEngineId": aqc_package_json_engine::ENGINE_ID,
        "publicGetterCounts": {"compilerOptions": 22, "scripts": 1, "devDependencies": 1},
    })
}

const fn jsonc_options() -> JsonParseOptions {
    JsonParseOptions {
        allow_comments: true,
        allow_loose_object_property_names: false,
        allow_trailing_commas: true,
        allow_missing_commas: false,
        allow_single_quoted_strings: false,
        allow_hexadecimal_numbers: true,
        allow_unary_plus_numbers: false,
        allow_extended_json_numbers: true,
        allow_extended_string_escapes: true,
        allow_extended_whitespace: true,
        allow_utf8_bom: true,
    }
}

fn main() {
    let fixture_path = std::env::args().nth(1).expect("fixture path");
    let fixture: Value =
        serde_json::from_slice(&std::fs::read(Path::new(&fixture_path)).expect("fixture bytes"))
            .expect("fixture JSON");
    let required_layers = [
        "jsonc-dialect",
        "jsonc-preservation",
        "jsonc-failures",
        "compiler-options",
        "package-json-keyed-maps",
        "merge-and-attribution",
        "absent-file-init",
        "downstream-public-api",
    ];
    assert_eq!(fixture["layers"], json!(required_layers));
    let output = json!({
        "jsoncDialect": jsonc_dialect(),
        "jsoncPreservation": jsonc_preservation(),
        "jsoncFailures": jsonc_failures(),
        "compilerOptions": compiler_options(&fixture),
        "packageJsonKeyedMaps": package_json_maps(),
        "mergeAndAttribution": merge_and_attribution(),
        "absentFileInit": absent_file_init(),
        "downstreamPublicApi": public_api(),
    });
    println!("{}", serde_json::to_string(&output).expect("output JSON"));
}
