#![expect(
    clippy::expect_used,
    clippy::panic,
    reason = "Contract tests use direct assertions for fixture invariants."
)]

use aqc_file_engine_core as _;
use aqc_jsonc_engine_core::{
    ConfigScalar, Finding, JsoncObject, JsoncParseOptions, Provenance, ResolvedRequirement,
    ScalarAssertion, parse_object_or_report, reconcile_scalar_assertion,
};
use jsonc_parser as _;
use tree_sitter as _;
use tree_sitter_javascript as _;

#[test]
fn accepted_dialect_and_unchanged_bytes_are_preserved_exactly() {
    let bytes =
        b"{\r\n  // kept\r\n  \"hex\": 0x10,\r\n  \"nested\": {\"enabled\": true,},\r\n}\r\n";
    let object = parse(bytes);
    assert_eq!(object.scalar(&["hex"]), Some(ConfigScalar::Int(16)));
    assert!(object.object_exists(&["nested"]));
    assert_eq!(
        object.render(),
        bytes,
        "An unchanged CST must preserve every byte."
    );
}

#[test]
fn javascript_number_extensions_and_bom_are_preserved_exactly() {
    let bytes = b"\xef\xbb\xbf{\n  \"binary\": 0b10,\n  \"octal\": 0o10,\n  \"leading\": .5,\n  \"trailing\": 1.,\n  \"separator\": 1_000,\n  \"negative\": -0b11,\n}\n";
    let object = parse(bytes);
    assert_eq!(object.scalar(&["binary"]), Some(ConfigScalar::Int(2)));
    assert_eq!(object.scalar(&["octal"]), Some(ConfigScalar::Int(8)));
    assert_eq!(
        object.scalar(&["separator"]),
        Some(ConfigScalar::Int(1_000))
    );
    assert_eq!(object.scalar(&["negative"]), Some(ConfigScalar::Int(-3)));
    assert_eq!(object.render(), bytes);
}

#[test]
fn edits_preserve_javascript_number_extensions_and_bom() {
    let bytes = b"\xef\xbb\xbf{\n  \"binary\": 0b10,\n  \"nested\": {},\n}\n";
    let mut object = parse(bytes);
    assert!(object.set_scalar(&["nested", "strict"], ConfigScalar::Bool(true)));
    let rendered = String::from_utf8(object.render()).expect("Rendered JSONC must be UTF-8.");
    assert!(rendered.starts_with('\u{feff}'));
    assert!(rendered.contains("\"binary\": 0b10"));
    assert!(rendered.contains("\"strict\": true"));
}

#[test]
fn configured_syntax_extensions_are_rejected() {
    for bytes in [
        b"{'value': true}".as_slice(),
        b"{value: true}".as_slice(),
        b"{\"a\": true \"b\": false}".as_slice(),
        b"{\"value\": +1}".as_slice(),
        b"{\"value\": 1n}".as_slice(),
    ] {
        let (object, findings) = parse_object_or_report(Some(bytes), "test.jsonc", options());
        assert!(
            object.is_none(),
            "The incompatible syntax must be rejected."
        );
        assert!(matches!(findings.as_slice(), [Finding::ParseError { .. }]));
    }
}

#[test]
fn invalid_inputs_and_non_object_roots_are_rejected_without_replacement() {
    for bytes in [
        b"".as_slice(),
        b"{".as_slice(),
        b"[]".as_slice(),
        b"null".as_slice(),
        &[0xff],
    ] {
        let (object, findings) = parse_object_or_report(Some(bytes), "test.jsonc", options());
        assert!(object.is_none());
        assert_eq!(findings.len(), 1);
    }
}

#[test]
fn duplicate_members_are_rejected_recursively_in_objects_and_arrays() {
    for bytes in [
        br#"{"a":1,"a":2}"#.as_slice(),
        br#"{"outer":{"a":1,"a":2}}"#.as_slice(),
        br#"{"items":[{"a":1,"a":2}]}"#.as_slice(),
        br#"{"items":[[{"a":1,"a":2}]]}"#.as_slice(),
    ] {
        let (object, findings) = parse_object_or_report(Some(bytes), "test.jsonc", options());
        assert!(object.is_none());
        match findings.as_slice() {
            [Finding::ParseError { message, .. }] => {
                assert!(message.contains("duplicate object member `a`"));
            }
            _ => panic!("A duplicate must produce one parse finding."),
        }
    }
}

#[test]
fn setters_preserve_comments_and_refuse_non_object_parents() {
    let bytes = b"{\n  // before\n  \"compiler\": {\n    \"kept\": 0x10, // inline\n  },\n  \"blocked\": false,\n}\n";
    let mut object = parse(bytes);
    assert!(object.set_scalar(&["compiler", "strict"], ConfigScalar::Bool(true)));
    assert!(!object.set_scalar(&["blocked", "strict"], ConfigScalar::Bool(true)));
    let rendered = String::from_utf8(object.render()).expect("Rendered JSONC must be UTF-8.");
    assert!(rendered.contains("// before"));
    assert!(rendered.contains("0x10, // inline"));
    assert!(rendered.contains("\"strict\": true"));
    assert!(rendered.contains("\"blocked\": false"));
}

#[test]
fn removal_is_targeted_and_requires_object_parents() {
    let mut object = parse(br#"{"nested":{"remove":true,"keep":false},"blocked":[]}"#);
    assert!(object.remove_value(&["nested", "remove"]));
    assert!(!object.remove_value(&["blocked", "remove"]));
    assert_eq!(
        object.scalar(&["nested", "keep"]),
        Some(ConfigScalar::Bool(false))
    );
    assert!(!object.value_exists(&["nested", "remove"]));
}

#[test]
fn scalar_reconciliation_reports_selector_attribution_and_writes() {
    let provenance = Provenance {
        policy: "policy".to_owned(),
    };
    let requirement = ResolvedRequirement {
        merged: ScalarAssertion::Equals(true, "Require true.".to_owned()),
        collected: vec![(
            provenance.clone(),
            ScalarAssertion::Equals(true, "Require true.".to_owned()),
        )],
    };
    let mut object = parse(br#"{"compiler":{"strict":false}}"#);
    let mut findings = Vec::new();
    reconcile_scalar_assertion(
        &mut object,
        &["compiler", "strict"],
        Some("strict".to_owned()),
        &requirement,
        |value| Some(ConfigScalar::Bool(*value)),
        |value| match value {
            ConfigScalar::Bool(value) => Some(value),
            ConfigScalar::Str(_) | ConfigScalar::Int(_) => None,
        },
        &mut findings,
    );
    assert!(matches!(
        findings.as_slice(),
        [Finding::Mismatch { selector: Some(selector), attribution, .. }]
            if selector == "strict" && attribution == &[provenance]
    ));
    assert_eq!(
        object.scalar(&["compiler", "strict"]),
        Some(ConfigScalar::Bool(true))
    );
}

fn parse(bytes: &[u8]) -> JsoncObject {
    let (object, findings) = parse_object_or_report(Some(bytes), "test.jsonc", options());
    assert!(findings.is_empty(), "Fixture must parse: {findings:?}");
    object.expect("Fixture must produce an object.")
}

const fn options() -> JsoncParseOptions {
    JsoncParseOptions {
        allow_comments: true,
        allow_loose_object_property_names: false,
        allow_trailing_commas: true,
        allow_missing_commas: false,
        allow_single_quoted_strings: false,
        allow_hexadecimal_numbers: true,
        allow_unary_plus_numbers: false,
        allow_extended_json_numbers: true,
        allow_utf8_bom: true,
    }
}
