#![expect(
    clippy::panic,
    reason = "The accepted contract requires this exact integration-test filename, which the repository classifier treats as production."
)]

use aqc_file_engine_core::{ResolvedRequirement, ScalarAssertion};
use aqc_json_engine_core::{
    ConfigScalar, Finding, Provenance, parse_object_or_report, reconcile_scalar_assertion,
    render_object,
};
use serde as _;
use serde_json as _;

#[test]
fn missing_object_renders_deterministic_json() {
    let (mut object, findings) = parse_object_or_report(None, "package.json");
    assert!(
        findings.is_empty(),
        "Missing bytes must parse as an empty object."
    );
    let object = object
        .as_mut()
        .expect("Missing bytes must return an object.");
    assert!(
        object.set_scalar(&["z"], ConfigScalar::Bool(true)),
        "A root scalar must be writable."
    );
    assert!(
        object.set_scalar(&["a", "value"], ConfigScalar::Str("x".to_owned())),
        "A nested scalar must be writable."
    );
    assert_eq!(
        render_object(object),
        b"{\n  \"a\": {\n    \"value\": \"x\"\n  },\n  \"z\": true\n}\n"
    );
}

#[test]
fn addressed_removal_preserves_sibling_values() {
    let (mut object, findings) = parse_object_or_report(
        Some(br#"{"devEngines":{"packageManager":{"name":"pnpm","kept":true}},"name":"kept"}"#),
        "package.json",
    );
    assert!(findings.is_empty(), "The fixture must parse.");
    let object = object.as_mut().expect("The fixture must return an object.");
    assert!(
        object.remove_value(&["devEngines", "packageManager", "name"]),
        "The addressed value must be removed."
    );
    assert_eq!(
        render_object(object),
        b"{\n  \"devEngines\": {\n    \"packageManager\": {\n      \"kept\": true\n    }\n  },\n  \"name\": \"kept\"\n}\n",
        "Removal must not modify sibling values."
    );
}

#[test]
fn duplicate_root_member_is_rejected_before_map_construction() {
    let (object, findings) = parse_object_or_report(
        Some(br#"{"packageManager":"pnpm@11","packageManager":"npm@10"}"#),
        "package.json",
    );
    assert!(
        object.is_none(),
        "A duplicate root member must reject the document."
    );
    assert_parse_error_mentions(&findings, "duplicate object member `packageManager`");
}

#[test]
fn duplicate_nested_member_is_rejected_before_map_construction() {
    let (object, findings) = parse_object_or_report(
        Some(br#"{"devEngines":{"packageManager":{"name":"pnpm","name":"npm"}}}"#),
        "package.json",
    );
    assert!(
        object.is_none(),
        "A duplicate nested member must reject the document."
    );
    assert_parse_error_mentions(&findings, "duplicate object member `name`");
}

#[test]
fn malformed_and_non_object_roots_report_one_shape_finding() {
    for bytes in [b"{".as_slice(), b"[]".as_slice(), b"null".as_slice()] {
        let (object, findings) = parse_object_or_report(Some(bytes), "package.json");
        assert!(
            object.is_none(),
            "Malformed or non-object JSON must not return an object."
        );
        assert_eq!(
            findings.len(),
            1,
            "Each parse failure must produce one finding."
        );
    }
}

#[test]
fn wrong_shape_is_distinct_from_a_missing_scalar() {
    let (object, findings) = parse_object_or_report(
        Some(br#"{"packageManager":{"name":"pnpm"}}"#),
        "package.json",
    );
    assert!(
        findings.is_empty(),
        "A valid object must parse without findings."
    );
    let object = object.expect("A valid object must be returned.");
    assert!(
        object.value_exists(&["packageManager"]),
        "The object value must exist."
    );
    assert_eq!(
        object.scalar(&["packageManager"]),
        None,
        "An object is not a scalar."
    );
    assert_eq!(
        object.rendered_value(&["missing"]),
        None,
        "A missing value has no rendering."
    );
}

#[test]
fn object_existence_requires_an_object_at_the_exact_path() {
    let (object, findings) = parse_object_or_report(
        Some(br#"{"object":{"nested":{}},"scalar":"value","array":[]}"#),
        "package.json",
    );
    assert!(findings.is_empty(), "A valid object must parse.");
    let object = object.expect("A valid object must be returned.");

    assert!(object.object_exists(&["object"]));
    assert!(object.object_exists(&["object", "nested"]));
    assert!(!object.object_exists(&["scalar"]));
    assert!(!object.object_exists(&["array"]));
    assert!(!object.object_exists(&["missing"]));
    assert!(!object.object_exists(&[]));
}

#[test]
fn generic_scalar_reconciliation_reads_writes_and_attributes() {
    let provenance = Provenance {
        policy: "policy".to_owned(),
    };
    let resolved = ResolvedRequirement {
        merged: ScalarAssertion::Equals("pnpm".to_owned(), "use pnpm".to_owned()),
        collected: vec![(
            provenance.clone(),
            ScalarAssertion::Equals("pnpm".to_owned(), "use pnpm".to_owned()),
        )],
    };
    let (mut object, parse_findings) =
        parse_object_or_report(Some(br#"{"manager":"npm"}"#), "test.json");
    assert!(parse_findings.is_empty());
    let mut object = object.take().expect("The object must parse.");
    let mut findings = Vec::new();
    reconcile_scalar_assertion(
        &mut object,
        &["manager"],
        &resolved,
        |value| Some(ConfigScalar::Str(value.clone())),
        |value| match value {
            ConfigScalar::Str(value) => Some(value),
            ConfigScalar::Bool(_) | ConfigScalar::Int(_) => None,
        },
        &mut findings,
    );
    assert!(matches!(
        findings.as_slice(),
        [Finding::Mismatch { attribution, .. }] if attribution == &[provenance]
    ));
    assert_eq!(
        object.scalar(&["manager"]),
        Some(ConfigScalar::Str("pnpm".to_owned()))
    );
}

fn assert_parse_error_mentions(findings: &[Finding], expected: &str) {
    match findings {
        [Finding::ParseError { message, .. }] => assert!(
            message.contains(expected),
            "The parse finding must identify the rejected shape."
        ),
        _ => panic!("The parser must emit exactly one parse finding."),
    }
}
