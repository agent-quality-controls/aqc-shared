#![expect(
    clippy::expect_used,
    clippy::panic,
    reason = "Contract tests use direct assertions for fixture invariants."
)]

use aqc_file_engine_core::{
    Engine, EngineRequirement, FileEngine, Finding, Provenance, ScalarAssertion,
};
use aqc_json_engine_core as _;
use aqc_tsconfig_json_engine::{
    ResolvedTsconfigJsonRequirements, TsconfigBooleanCompilerOption, TsconfigJsonEngine,
    TsconfigJsonRequirements,
};
use schemars as _;
use serde as _;
use std::fmt::Write as _;

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

#[test]
fn exact_option_vocabulary_maps_to_unique_file_keys() {
    let keys = OPTIONS
        .iter()
        .map(|(option, expected, _)| {
            assert_eq!(option.file_key(), *expected);
            option.file_key()
        })
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(
        keys.len(),
        22,
        "Every option must have one unique file key."
    );
}

#[test]
fn absent_file_initializes_all_options_and_reaches_a_fixed_point() {
    let resolved = baseline();
    let output = <TsconfigJsonEngine as FileEngine<_>>::reconcile(None, &resolved);
    assert_eq!(output.findings.len(), 22);
    let rendered = String::from_utf8(output.expected_bytes.clone()).expect("Output must be UTF-8.");
    for (_, key, value) in OPTIONS {
        assert!(
            rendered.contains(&format!("\"{key}\": {value}")),
            "missing {key}"
        );
    }
    let second =
        <TsconfigJsonEngine as FileEngine<_>>::reconcile(Some(&output.expected_bytes), &resolved);
    assert!(second.findings.is_empty());
    assert_eq!(second.expected_bytes, output.expected_bytes);
}

#[test]
fn valid_existing_jsonc_is_an_exact_no_op() {
    let bytes = baseline_bytes_with_comments();
    let output = <TsconfigJsonEngine as FileEngine<_>>::reconcile(Some(&bytes), &baseline());
    assert!(output.findings.is_empty());
    assert_eq!(output.expected_bytes, bytes);
}

#[test]
fn corrections_preserve_comments_trailing_commas_and_unmanaged_values() {
    let bytes = b"{\n  // root\n  \"extends\": \"./base.json\",\n  \"compilerOptions\": {\n    // option\n    \"strict\": false,\n    \"module\": \"preserve\",\n  },\n}\n";
    let resolved = merge(one_option(TsconfigBooleanCompilerOption::Strict, true));
    let output = <TsconfigJsonEngine as FileEngine<_>>::reconcile(Some(bytes), &resolved);
    assert_eq!(output.findings.len(), 1);
    let rendered = String::from_utf8(output.expected_bytes).expect("Output must be UTF-8.");
    assert!(rendered.contains("// root"));
    assert!(rendered.contains("// option"));
    assert!(rendered.contains("\"module\": \"preserve\","));
    assert!(rendered.contains("\"strict\": true,"));
}

#[test]
fn typescript_syntax_extensions_survive_reconciliation() {
    let bytes = b"\xef\xbb\xbf{\n  // preserved\n  \"compilerOptions\": {\n    \"strict\": false,\n    \"hex\": 0x10,\n    \"binary\": 0b10,\n    \"octal\": 0o10,\n    \"leading\": .5,\n    \"trailing\": 1.,\n    \"separator\": 1_000,\n    \"negative\": -0b11,\n  },\n}\n";
    let expected = b"\xef\xbb\xbf{\n  // preserved\n  \"compilerOptions\": {\n    \"strict\": true,\n    \"hex\": 0x10,\n    \"binary\": 0b10,\n    \"octal\": 0o10,\n    \"leading\": .5,\n    \"trailing\": 1.,\n    \"separator\": 1_000,\n    \"negative\": -0b11,\n  },\n}\n";
    let resolved = merge(one_option(TsconfigBooleanCompilerOption::Strict, true));
    let output = <TsconfigJsonEngine as FileEngine<_>>::reconcile(Some(bytes), &resolved);
    assert_eq!(output.findings.len(), 1);
    assert_eq!(output.expected_bytes, expected);
}

#[test]
fn typescript_string_escapes_survive_reconciliation() {
    let bytes = b"{\x0b\n  \"label\": \"\\x41\\v\\0\\'\\q\\u{1f600}line\\\ncontinued\",\n  \"compilerOptions\": { \"strict\": false },\n}\n";
    let expected = b"{\x0b\n  \"label\": \"\\x41\\v\\0\\'\\q\\u{1f600}line\\\ncontinued\",\n  \"compilerOptions\": { \"strict\": true },\n}\n";
    let resolved = merge(one_option(TsconfigBooleanCompilerOption::Strict, true));
    let output = <TsconfigJsonEngine as FileEngine<_>>::reconcile(Some(bytes), &resolved);
    assert_eq!(output.findings.len(), 1);
    assert_eq!(output.expected_bytes, expected);
}

#[test]
fn unsupported_typescript_syntax_extensions_are_rejected_and_preserved() {
    for bytes in [
        b"{compilerOptions:{\"strict\":true}}".as_slice(),
        b"{\"compilerOptions\":{\"strict\":true \"noCheck\":false}}".as_slice(),
        b"{'compilerOptions':{\"strict\":true}}".as_slice(),
        b"{\"compilerOptions\":{\"strict\":true},\"value\":+1}".as_slice(),
    ] {
        let output = <TsconfigJsonEngine as FileEngine<_>>::reconcile(Some(bytes), &baseline());
        assert_eq!(output.expected_bytes, bytes);
        assert!(matches!(
            output.findings.as_slice(),
            [Finding::ParseError { .. }]
        ));
    }
}

#[test]
fn wrong_compiler_options_shape_is_reported_once_and_preserved() {
    for bytes in [
        b"{\"compilerOptions\":null}".as_slice(),
        b"{\"compilerOptions\":[]}".as_slice(),
    ] {
        let output = <TsconfigJsonEngine as FileEngine<_>>::reconcile(Some(bytes), &baseline());
        assert_eq!(output.expected_bytes, bytes);
        assert!(matches!(
            output.findings.as_slice(),
            [Finding::Mismatch { key, selector: None, attribution, .. }]
                if key == "compilerOptions" && attribution == &[provenance("policy")]
        ));
    }
}

#[test]
fn parse_failures_preserve_existing_bytes_and_suppress_option_findings() {
    for bytes in [
        b"{".as_slice(),
        b"[]".as_slice(),
        br#"{"compilerOptions":{"strict":true,"strict":false}}"#.as_slice(),
    ] {
        let output = <TsconfigJsonEngine as FileEngine<_>>::reconcile(Some(bytes), &baseline());
        assert_eq!(output.expected_bytes, bytes);
        assert!(matches!(
            output.findings.as_slice(),
            [Finding::ParseError { .. }]
        ));
    }
}

#[test]
fn scalar_check_only_and_absent_assertions_use_core_algebra() {
    let requirements = TsconfigJsonRequirements {
        boolean_compiler_options: [
            (
                TsconfigBooleanCompilerOption::Strict,
                ScalarAssertion::Present("Declare strict.".to_owned()),
            ),
            (
                TsconfigBooleanCompilerOption::NoCheck,
                ScalarAssertion::OneOf(
                    std::iter::once(false).collect(),
                    "Disable noCheck.".to_owned(),
                ),
            ),
            (
                TsconfigBooleanCompilerOption::AllowUnusedLabels,
                ScalarAssertion::Absent("Remove the option.".to_owned()),
            ),
        ]
        .into_iter()
        .collect(),
    };
    let resolved = merge(requirements);
    let bytes = br#"{"compilerOptions":{"allowUnusedLabels":true,"module":"preserve"}}"#;
    let output = <TsconfigJsonEngine as FileEngine<_>>::reconcile(Some(bytes), &resolved);
    assert_eq!(output.findings.len(), 3);
    let rendered = String::from_utf8(output.expected_bytes).expect("Output must be UTF-8.");
    assert!(!rendered.contains("allowUnusedLabels"));
    assert!(!rendered.contains("\"strict\""));
    assert!(!rendered.contains("\"noCheck\""));
    assert!(rendered.contains("\"module\""));
    assert!(rendered.contains("\"preserve\""));
}

#[test]
fn option_mismatches_are_independent_and_use_file_key_selectors() {
    let resolved = baseline();
    let output = <TsconfigJsonEngine as FileEngine<_>>::reconcile(
        Some(b"{\"compilerOptions\":{\"strict\":\"yes\",\"noCheck\":true}}"),
        &resolved,
    );
    assert_eq!(output.findings.len(), 22);
    for finding in output.findings {
        let Finding::Mismatch {
            key,
            selector: Some(selector),
            attribution,
            ..
        } = finding
        else {
            panic!("Each option must produce a selected mismatch.");
        };
        assert_eq!(key, format!("compilerOptions.{selector}"));
        assert_eq!(attribution, vec![provenance("policy")]);
    }
}

#[test]
fn equal_requirements_retain_attribution_and_opposites_conflict() {
    let option = TsconfigBooleanCompilerOption::Strict;
    let equal = TsconfigJsonRequirements::merge(vec![
        (provenance("a"), one_option(option, true)),
        (provenance("b"), one_option(option, true)),
    ])
    .expect("Equal requirements must merge.");
    assert_eq!(
        equal
            .boolean_compiler_options()
            .get(&option)
            .expect("Option must resolve.")
            .collected
            .len(),
        2
    );
    let conflicts = TsconfigJsonRequirements::merge(vec![
        (provenance("a"), one_option(option, true)),
        (provenance("b"), one_option(option, false)),
    ])
    .expect_err("Opposite requirements must conflict.");
    assert_eq!(conflicts.len(), 1);
    assert_eq!(
        conflicts.first().expect("The conflict must exist.").key,
        "compilerOptions.strict"
    );
}

#[test]
fn ordered_boolean_assertions_use_core_conflict_behavior() {
    let option = TsconfigBooleanCompilerOption::Strict;
    let requirements = TsconfigJsonRequirements {
        boolean_compiler_options: std::iter::once((
            option,
            ScalarAssertion::AtLeast(true, "Ordered booleans are unsupported.".to_owned()),
        ))
        .collect(),
    };
    let conflicts = TsconfigJsonRequirements::merge(vec![(provenance("policy"), requirements)])
        .expect_err("Ordered booleans must fail during resolution.");
    assert_eq!(conflicts.len(), 1);
    assert_eq!(
        conflicts.first().expect("The conflict must exist.").reason,
        "scalar-order-unsupported"
    );
}

#[test]
fn erased_engine_dispatch_stops_on_merge_conflicts() {
    let requirements: Vec<(Provenance, Box<dyn EngineRequirement>)> = vec![
        (
            provenance("a"),
            Box::new(one_option(TsconfigBooleanCompilerOption::Strict, true)),
        ),
        (
            provenance("b"),
            Box::new(one_option(TsconfigBooleanCompilerOption::Strict, false)),
        ),
    ];
    let output = Engine::reconcile(&TsconfigJsonEngine, None, &requirements);
    assert!(matches!(
        output.findings.as_slice(),
        [Finding::ConflictingRequirements { .. }]
    ));
}

fn baseline() -> ResolvedTsconfigJsonRequirements {
    let boolean_compiler_options = OPTIONS
        .iter()
        .map(|(option, _, value)| {
            (
                *option,
                ScalarAssertion::Equals(*value, format!("Require {}.", option.file_key())),
            )
        })
        .collect();
    merge(TsconfigJsonRequirements {
        boolean_compiler_options,
    })
}

fn baseline_bytes_with_comments() -> Vec<u8> {
    let mut text = String::from("{\n  // preserved\n  \"compilerOptions\": {\n");
    let mut options = OPTIONS.iter().peekable();
    while let Some((_, key, value)) = options.next() {
        write!(text, "    \"{key}\": {value}").expect("Writing to a String must succeed.");
        if options.peek().is_some() {
            text.push(',');
        }
        text.push('\n');
    }
    text.push_str("  }\n}\n");
    text.into_bytes()
}

fn one_option(option: TsconfigBooleanCompilerOption, value: bool) -> TsconfigJsonRequirements {
    TsconfigJsonRequirements {
        boolean_compiler_options: std::iter::once((
            option,
            ScalarAssertion::Equals(value, format!("Require {}.", option.file_key())),
        ))
        .collect(),
    }
}

fn merge(requirement: TsconfigJsonRequirements) -> ResolvedTsconfigJsonRequirements {
    TsconfigJsonRequirements::merge(vec![(provenance("policy"), requirement)])
        .expect("Compatible requirements must merge.")
}

fn provenance(policy: &str) -> Provenance {
    Provenance {
        policy: policy.to_owned(),
    }
}
