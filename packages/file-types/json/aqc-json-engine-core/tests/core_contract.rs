#![expect(
    clippy::expect_used,
    clippy::panic,
    reason = "Contract tests use direct assertions for fixture invariants."
)]

use aqc_file_engine_core as _;
use aqc_json_engine_core::{
    ConfigScalar, Finding, JsonObject, JsonParseOptions, NonObjectParentAction, Provenance,
    ResolvedRequirement, ScalarAssertion, parse_object_or_report, reconcile_scalar_assertion,
};
use jsonc_parser as _;
use serde_json as _;
use tree_sitter as _;
use tree_sitter_javascript as _;

#[test]
fn absent_document_renders_deterministic_empty_object() {
    let (object, findings) = parse_object_or_report(None, "test.json", strict_options());
    assert!(
        findings.is_empty(),
        "Absent input must parse without findings."
    );
    assert_eq!(
        object
            .expect("Absent input must produce an object.")
            .render(),
        b"{}\n"
    );
}

#[test]
fn strict_json_preserves_unchanged_bytes_exactly() {
    let bytes = b"{\r\n  \"string\": \"kept\",\r\n  \"integer\": -10,\r\n  \"float\": 1.25,\r\n  \"boolean\": true,\r\n  \"null\": null,\r\n  \"array\": [1, 2],\r\n  \"object\": {}\r\n}\r\n";
    let (object, findings) = parse_object_or_report(Some(bytes), "test.json", strict_options());
    assert!(
        findings.is_empty(),
        "Strict JSON fixture must parse: {findings:?}"
    );
    assert_eq!(
        object
            .expect("Strict JSON fixture must produce an object.")
            .render(),
        bytes
    );
}

#[test]
fn strict_json_rejects_every_supported_syntax_extension() {
    for bytes in [
        b"{/* comment */\"value\":true}".as_slice(),
        b"{value:true}".as_slice(),
        b"{\"value\":true,}".as_slice(),
        b"{\"first\":true \"second\":false}".as_slice(),
        b"{'value':true}".as_slice(),
        b"{\"value\":0x10}".as_slice(),
        b"{\"value\":+1}".as_slice(),
        b"{\"value\":0b10}".as_slice(),
        b"\xef\xbb\xbf{\"value\":true}".as_slice(),
    ] {
        let (object, findings) = parse_object_or_report(Some(bytes), "test.json", strict_options());
        assert!(
            object.is_none(),
            "Strict JSON must reject extension syntax: {bytes:?}"
        );
        assert!(matches!(findings.as_slice(), [Finding::ParseError { .. }]));
    }
}

#[test]
fn strict_json_rejects_non_json_characters_and_preserves_parser_messages() {
    for bytes in [
        b"{\x0c\"value\":true}".as_slice(),
        b"{\"value\":\"first\nsecond\"}".as_slice(),
    ] {
        let (object, findings) = parse_object_or_report(Some(bytes), "test.json", strict_options());
        assert!(object.is_none());
        assert!(matches!(findings.as_slice(), [Finding::ParseError { .. }]));
    }
    let (_, findings) =
        parse_object_or_report(Some(br#"{"value":}"#), "test.json", strict_options());
    assert!(matches!(
        findings.as_slice(),
        [Finding::ParseError { message, .. }]
            if message == "test.json is not a valid JSON object: expected value at line 1 column 10"
    ));
}

#[test]
fn each_syntax_extension_is_accepted_only_when_enabled() {
    let cases: &[(&[u8], fn(&mut JsonParseOptions))] = &[
        (b"{/* comment */\"value\":true}", |options| {
            options.allow_comments = true;
        }),
        (b"{value:true}", |options| {
            options.allow_loose_object_property_names = true;
        }),
        (b"{\"value\":true,}", |options| {
            options.allow_trailing_commas = true;
        }),
        (b"{\"first\":true \"second\":false}", |options| {
            options.allow_missing_commas = true;
        }),
        (b"{'value':true}", |options| {
            options.allow_single_quoted_strings = true;
        }),
        (b"{\"value\":0x10}", |options| {
            options.allow_hexadecimal_numbers = true;
        }),
        (b"{\"value\":+1}", |options| {
            options.allow_unary_plus_numbers = true;
        }),
        (b"{\"value\":0b10}", |options| {
            options.allow_extended_json_numbers = true;
        }),
        (b"\xef\xbb\xbf{\"value\":true}", |options| {
            options.allow_utf8_bom = true;
        }),
    ];
    for (bytes, enable) in cases {
        let mut options = strict_options();
        enable(&mut options);
        let (object, findings) = parse_object_or_report(Some(bytes), "test.json", options);
        assert!(
            findings.is_empty(),
            "Enabled syntax must parse: {bytes:?}: {findings:?}"
        );
        assert_eq!(
            object
                .expect("Enabled syntax must produce an object.")
                .render(),
            *bytes
        );
    }
}

#[test]
fn enabled_syntax_extensions_compose() {
    let bytes = b"{\"first\":0b1 \"second\":0o2}";
    let mut options = strict_options();
    options.allow_missing_commas = true;
    options.allow_extended_json_numbers = true;
    let (object, findings) = parse_object_or_report(Some(bytes), "test.jsonc", options);
    assert!(
        findings.is_empty(),
        "Enabled syntax extensions must compose: {findings:?}"
    );
    let object = object.expect("Combined extensions must produce an object.");
    assert_eq!(object.scalar(&["first"]), Some(ConfigScalar::Int(1)));
    assert_eq!(object.scalar(&["second"]), Some(ConfigScalar::Int(2)));
    assert_eq!(object.render(), bytes);
}

#[test]
fn jsonc_rejects_unescaped_control_characters_in_strings() {
    let bytes = b"{\"value\":\"first\nsecond\"}";
    let (object, findings) = parse_object_or_report(Some(bytes), "test.jsonc", options());
    assert!(object.is_none());
    assert!(matches!(findings.as_slice(), [Finding::ParseError { .. }]));
}

#[test]
fn extended_string_escapes_decode_and_preserve_exact_bytes() {
    let bytes = b"{\"plain\":\"same\",\"\\x76alue\":\"caf\xc3\xa9 \\x41\\vline\\\ncontinued\",\"crlf\":\"first\\\r\nsecond\",\"number\":0b1}";
    let object = parse(bytes);
    assert_eq!(
        object.scalar(&["value"]),
        Some(ConfigScalar::Str(
            "caf\u{e9} A\u{b}linecontinued".to_owned()
        ))
    );
    assert_eq!(
        object.scalar(&["crlf"]),
        Some(ConfigScalar::Str("firstsecond".to_owned()))
    );
    assert_eq!(object.scalar(&["number"]), Some(ConfigScalar::Int(1)));
    assert_eq!(object.render(), bytes);
}

#[test]
fn typescript_javascript_escape_set_decodes_and_preserves_exact_bytes() {
    let bytes = b"{\"zero\":\"\\0\",\"apostrophe\":\"\\'\",\"identity\":\"\\q\",\"unicode_identity\":\"\\\xc3\xa9\",\"separator\":\"first\\\xe2\x80\xa8second\",\"codepoint\":\"\\u{1f600}\",\"surrogate\":\"\\u{d800}\",\"standard_surrogate\":\"\\uD800\"}";
    let object = parse(bytes);
    assert_eq!(
        object.scalar(&["zero"]),
        Some(ConfigScalar::Str("\0".to_owned()))
    );
    assert_eq!(
        object.scalar(&["apostrophe"]),
        Some(ConfigScalar::Str("'".to_owned()))
    );
    assert_eq!(
        object.scalar(&["identity"]),
        Some(ConfigScalar::Str("q".to_owned()))
    );
    assert_eq!(
        object.scalar(&["unicode_identity"]),
        Some(ConfigScalar::Str("\u{e9}".to_owned()))
    );
    assert_eq!(
        object.scalar(&["separator"]),
        Some(ConfigScalar::Str("firstsecond".to_owned()))
    );
    assert_eq!(
        object.scalar(&["codepoint"]),
        Some(ConfigScalar::Str("\u{1f600}".to_owned()))
    );
    assert_eq!(object.scalar(&["surrogate"]), None);
    assert_eq!(object.scalar(&["standard_surrogate"]), None);
    assert_eq!(object.render(), bytes);
}

#[test]
fn typescript_rejects_octal_and_decimal_digit_string_escapes() {
    for escape in ["\\00", "\\01", "\\1", "\\08", "\\8", "\\9"] {
        let bytes = format!("{{\"value\":\"{escape}\"}}");
        let (object, findings) =
            parse_object_or_report(Some(bytes.as_bytes()), "test.jsonc", options());

        assert!(object.is_none(), "Escape must be rejected: {escape}");
        assert!(matches!(findings.as_slice(), [Finding::ParseError { .. }]));
    }
}

#[test]
fn replacing_an_unrepresentable_string_discards_its_mask_metadata() {
    let bytes = br#"{"value":"\uD800"}"#;
    let replacement = format!("{}d800", "_".repeat(bytes.len().saturating_add(1)));
    let mut object = parse(bytes);

    assert!(object.set_scalar(
        &["value"],
        ConfigScalar::Str(replacement.clone()),
        NonObjectParentAction::Preserve,
    ));
    assert_eq!(
        object.scalar(&["value"]),
        Some(ConfigScalar::Str(replacement))
    );
}

#[test]
fn string_lists_are_read_and_written_without_replacing_non_object_parents() {
    let mut object = parse(br#"{"nested":{"items":["a","b"]},"blocked":1}"#);
    assert_eq!(
        object.string_list(&["nested", "items"]),
        Some(vec!["a".to_owned(), "b".to_owned()])
    );
    assert!(object.value_is_array(&["nested", "items"]));
    assert!(!object.set_string_list(
        &["blocked", "items"],
        &["c".to_owned()],
        NonObjectParentAction::Preserve,
    ));
    assert_eq!(object.scalar(&["blocked"]), Some(ConfigScalar::Int(1)));
    assert!(object.set_string_list(
        &["nested", "items"],
        &["c".to_owned(), "d".to_owned()],
        NonObjectParentAction::Preserve,
    ));
    assert_eq!(
        object.string_list(&["nested", "items"]),
        Some(vec!["c".to_owned(), "d".to_owned()])
    );
}

#[test]
fn string_list_rejects_arrays_with_non_string_members() {
    let object = parse(br#"{"items":["a",1]}"#);
    assert!(object.value_is_array(&["items"]));
    assert_eq!(object.string_list(&["items"]), None);
}

#[test]
fn replacing_masked_parent_discards_stale_metadata() {
    let mut object = parse(br#"{"parent":0b1}"#);
    assert!(object.set_string_list(
        &["parent", "items"],
        &["value".to_owned()],
        NonObjectParentAction::Replace,
    ));
    assert_eq!(
        object.rendered_value(&["parent"]),
        Some("{\n    \"items\": [\"value\"]\n  }".to_owned())
    );
    assert_eq!(
        object.string_list(&["parent", "items"]),
        Some(vec!["value".to_owned()])
    );
}

#[test]
fn extended_string_normalization_errors_report_source_locations() {
    let bytes = b"{\n\"value\":\"\\u{110000}\"\n}";
    let (object, findings) = parse_object_or_report(Some(bytes), "test.jsonc", options());

    assert!(object.is_none());
    assert!(
        matches!(
            findings.as_slice(),
            [Finding::ParseError { message, .. }]
                if message.ends_with("invalid Unicode code point escape at line 2 column 10")
        ),
        "Unexpected findings: {findings:?}"
    );
}

#[test]
fn equivalent_unicode_escape_keys_are_duplicates() {
    let bytes = br#"{"\u{10000}":1,"\uD800\uDC00":2}"#;
    let (object, findings) = parse_object_or_report(Some(bytes), "test.jsonc", options());

    assert!(object.is_none());
    assert!(
        matches!(findings.as_slice(), [finding] if format!("{finding:?}").contains("duplicate")),
        "Unexpected findings: {findings:?}"
    );
}

#[test]
fn extended_whitespace_is_independent_and_preserved() {
    let bytes = b"{\x0b\"value\":true,\x0c\"other\":false,\xc2\xa0\"third\":true,\xe2\x80\xa8\"fourth\":false}";
    let options = JsonParseOptions {
        allow_extended_whitespace: true,
        ..strict_options()
    };
    let (object, findings) = parse_object_or_report(Some(bytes), "test.jsonc", options);
    assert!(findings.is_empty());
    assert_eq!(
        object.expect("Extended whitespace must parse.").render(),
        bytes
    );
}

#[test]
fn extended_whitespace_filter_respects_strings_and_comments() {
    let strict = b"{\"value\":\"before\xc2\xa0after\"}";
    let (strict_object, strict_findings) =
        parse_object_or_report(Some(strict), "test.json", strict_options());
    assert!(strict_findings.is_empty());
    assert_eq!(
        strict_object.expect("Strict JSON must parse.").render(),
        strict
    );

    let comments = b"{// before\xc2\xa0after\n\"value\":true}";
    let mut options = strict_options();
    options.allow_comments = true;
    let (comment_object, comment_findings) =
        parse_object_or_report(Some(comments), "test.jsonc", options);
    assert!(comment_findings.is_empty());
    assert_eq!(
        comment_object.expect("JSONC comment must parse.").render(),
        comments
    );
}

#[test]
fn extended_strings_do_not_collide_with_number_markers_or_comment_text() {
    let bytes = b"{// comment \\x41 must remain text\n\"text\":\"\\x5f\\x5fAQC_JSON_NUMBER_0__\",\"value\":0b1,\"later\":\"\\x42\"}";
    let object = parse(bytes);
    assert_eq!(
        object.scalar(&["text"]),
        Some(ConfigScalar::Str("__AQC_JSON_NUMBER_0__".to_owned()))
    );
    assert_eq!(object.scalar(&["value"]), Some(ConfigScalar::Int(1)));
    assert_eq!(
        object.scalar(&["later"]),
        Some(ConfigScalar::Str("B".to_owned()))
    );
    assert_eq!(object.render(), bytes);
}

#[test]
fn unrelated_syntax_options_do_not_enable_invalid_whitespace() {
    let options = JsonParseOptions {
        allow_utf8_bom: true,
        ..strict_options()
    };
    for bytes in [
        b"{\x0b\"value\":true}".as_slice(),
        b"{\x0c\"value\":true}".as_slice(),
    ] {
        let (object, findings) = parse_object_or_report(Some(bytes), "test.json", options);
        assert!(object.is_none());
        assert!(matches!(findings.as_slice(), [Finding::ParseError { .. }]));
    }
}

#[test]
fn extended_string_errors_are_classified_as_jsonc() {
    let options = JsonParseOptions {
        allow_extended_string_escapes: true,
        ..strict_options()
    };
    let (_, findings) = parse_object_or_report(Some(b"{"), "test.jsonc", options);
    assert!(matches!(
        findings.as_slice(),
        [Finding::ParseError { message, .. }] if message.starts_with("test.jsonc is not a valid JSONC object:")
    ));
}

#[test]
fn hexadecimal_separators_require_hexadecimal_and_extended_number_options() {
    let bytes = b"{\"value\":0x1_0}";
    for options in [
        JsonParseOptions {
            allow_hexadecimal_numbers: true,
            ..strict_options()
        },
        JsonParseOptions {
            allow_extended_json_numbers: true,
            ..strict_options()
        },
    ] {
        let (object, findings) = parse_object_or_report(Some(bytes), "test.jsonc", options);
        assert!(object.is_none());
        assert!(matches!(findings.as_slice(), [Finding::ParseError { .. }]));
    }
    let object = parse(bytes);
    assert_eq!(object.scalar(&["value"]), Some(ConfigScalar::Int(16)));
    assert_eq!(object.render(), bytes);
}

#[test]
fn extended_numbers_reject_invalid_numeric_separator_positions() {
    for number in ["0_1", "1__0", "1_e2", "1e_2", "0x_1", "0x1_"] {
        let bytes = format!("{{\"value\":{number}}}");
        let (object, findings) =
            parse_object_or_report(Some(bytes.as_bytes()), "test.jsonc", options());
        assert!(
            object.is_none(),
            "Invalid number must not be masked: {number}"
        );
        assert!(matches!(findings.as_slice(), [Finding::ParseError { .. }]));
    }
    let (object, findings) = parse_object_or_report(
        Some(b"{foo1_bar:true}"),
        "test.jsonc",
        JsonParseOptions {
            allow_loose_object_property_names: true,
            ..options()
        },
    );
    assert!(
        findings.is_empty(),
        "A loose property name is not a number."
    );
    assert!(object.is_some());
}

#[test]
fn extended_numbers_cannot_turn_numeric_object_keys_into_strings() {
    for bytes in [b"{0b1:true}".as_slice(), b"{[0b1]:true}".as_slice()] {
        let (object, findings) = parse_object_or_report(Some(bytes), "test.jsonc", options());
        assert!(object.is_none());
        assert!(matches!(findings.as_slice(), [Finding::ParseError { .. }]));
    }
}

#[test]
fn unary_plus_integer_is_available_through_scalar_access() {
    let mut options = strict_options();
    options.allow_unary_plus_numbers = true;
    let (object, findings) = parse_object_or_report(Some(b"{\"value\":+1}"), "test.jsonc", options);
    assert!(
        findings.is_empty(),
        "Unary-plus fixture must parse: {findings:?}"
    );
    let object = object.expect("Unary-plus fixture must produce an object.");
    assert_eq!(object.scalar(&["value"]), Some(ConfigScalar::Int(1)));
    assert_eq!(object.render(), b"{\"value\":+1}");
}

#[test]
fn rendered_compound_value_restores_extended_number_spelling() {
    let object = parse(br#"{"value":[0b1,{"nested":0o2}]}"#);
    assert_eq!(
        object.rendered_value(&["value"]),
        Some("[0b1,{\"nested\":0o2}]".to_owned())
    );
}

#[test]
fn escaped_user_string_cannot_impersonate_an_internal_number_marker() {
    let bytes = br#"{"text":"\u005f\u005fAQC_JSON_NUMBER_0__","value":0b1}"#;
    let object = parse(bytes);
    assert_eq!(
        object.scalar(&["text"]),
        Some(ConfigScalar::Str("__AQC_JSON_NUMBER_0__".to_owned()))
    );
    assert_eq!(object.scalar(&["value"]), Some(ConfigScalar::Int(1)));
    assert_eq!(object.render(), bytes);
}

#[test]
fn masked_numbers_do_not_shift_parse_or_duplicate_diagnostics() {
    let malformed = br#"{"value":0b1,"broken":}"#;
    let (malformed_object, malformed_findings) =
        parse_object_or_report(Some(malformed), "test.jsonc", options());
    assert!(malformed_object.is_none());
    let malformed_column = std::str::from_utf8(malformed)
        .expect("Fixture must be UTF-8.")
        .trim_end_matches('}')
        .chars()
        .count()
        .saturating_add(1);
    assert!(matches!(
        malformed_findings.as_slice(),
        [Finding::ParseError { message, .. }]
            if message.ends_with(&format!("line 1 column {malformed_column}"))
    ));

    let duplicate = br#"{"value":0b1,"duplicate":1,"duplicate":2}"#;
    let duplicate_text = std::str::from_utf8(duplicate).expect("Fixture must be UTF-8.");
    let duplicate_end = duplicate_text
        .rfind("\"duplicate\"")
        .expect("Fixture must contain the duplicate key.")
        .saturating_add("\"duplicate\"".len());
    let duplicate_column = duplicate_text
        .get(..duplicate_end)
        .expect("Duplicate key boundary must be valid UTF-8.")
        .chars()
        .count();
    let (duplicate_object, duplicate_findings) =
        parse_object_or_report(Some(duplicate), "test.jsonc", options());
    assert!(duplicate_object.is_none());
    assert!(matches!(
        duplicate_findings.as_slice(),
        [Finding::ParseError { message, .. }]
            if message.ends_with(&format!("line 1 column {duplicate_column}"))
    ));
}

#[test]
fn masked_strings_do_not_shift_parse_or_duplicate_diagnostics() {
    let malformed = br#"{"text":"\u{41}","broken":}"#;
    let (malformed_object, malformed_findings) =
        parse_object_or_report(Some(malformed), "test.jsonc", options());
    assert!(malformed_object.is_none());
    let malformed_column = std::str::from_utf8(malformed)
        .expect("Fixture must be UTF-8.")
        .trim_end_matches('}')
        .chars()
        .count()
        .saturating_add(1);
    assert!(matches!(
        malformed_findings.as_slice(),
        [Finding::ParseError { message, .. }]
            if message.ends_with(&format!("line 1 column {malformed_column}"))
    ));

    let duplicate = br#"{"text":"\v","duplicate":1,"duplicate":2}"#;
    let duplicate_text = std::str::from_utf8(duplicate).expect("Fixture must be UTF-8.");
    let duplicate_end = duplicate_text
        .rfind("\"duplicate\"")
        .expect("Fixture must contain the duplicate key.")
        .saturating_add("\"duplicate\"".len());
    let duplicate_column = duplicate_text
        .get(..duplicate_end)
        .expect("Duplicate key boundary must be valid UTF-8.")
        .chars()
        .count();
    let (duplicate_object, duplicate_findings) =
        parse_object_or_report(Some(duplicate), "test.jsonc", options());
    assert!(duplicate_object.is_none());
    assert!(matches!(
        duplicate_findings.as_slice(),
        [Finding::ParseError { message, .. }]
            if message.ends_with(&format!("line 1 column {duplicate_column}"))
    ));
}

#[test]
fn duplicate_diagnostics_count_cr_and_crlf_line_endings() {
    for bytes in [
        b"{\r\"value\":1,\r\"value\":2}".as_slice(),
        b"{\r\n\"value\":1,\r\n\"value\":2}".as_slice(),
    ] {
        let (_, findings) = parse_object_or_report(Some(bytes), "test.jsonc", options());
        assert!(matches!(
            findings.as_slice(),
            [Finding::ParseError { message, .. }]
                if message.ends_with("duplicate object member `value` at line 3 column 7")
        ));
    }
}

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
    assert!(object.set_scalar(
        &["nested", "strict"],
        ConfigScalar::Bool(true),
        NonObjectParentAction::Preserve
    ));
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
    for options in [strict_options(), options()] {
        for bytes in [
            br#"{"a":1,"a":2}"#.as_slice(),
            br#"{"outer":{"a":1,"a":2}}"#.as_slice(),
            br#"{"items":[{"a":1,"a":2}]}"#.as_slice(),
            br#"{"items":[[{"a":1,"a":2}]]}"#.as_slice(),
        ] {
            let (object, findings) = parse_object_or_report(Some(bytes), "test.json", options);
            assert!(object.is_none());
            match findings.as_slice() {
                [Finding::ParseError { message, .. }] => {
                    assert!(
                        message.contains("duplicate object member `a`"),
                        "Unexpected duplicate diagnostic: {message}"
                    );
                }
                _ => panic!("A duplicate must produce one parse finding."),
            }
        }
    }
}

#[test]
fn setters_preserve_comments() {
    let bytes = b"{\n  // before\n  \"compiler\": {\n    \"kept\": 0x10, // inline\n  },\n  \"blocked\": false,\n}\n";
    let mut object = parse(bytes);
    assert!(object.set_scalar(
        &["compiler", "strict"],
        ConfigScalar::Bool(true),
        NonObjectParentAction::Preserve
    ));
    let rendered = String::from_utf8(object.render()).expect("Rendered JSONC must be UTF-8.");
    assert!(rendered.contains("// before"));
    assert!(rendered.contains("0x10, // inline"));
    assert!(rendered.contains("\"strict\": true"));
    assert!(rendered.contains("\"blocked\": false"));
}

#[test]
fn set_scalar_refuses_to_replace_an_existing_non_object_parent() {
    let bytes = br#"{"blocked":false,"kept":true}"#;
    let (object, findings) = parse_object_or_report(Some(bytes), "test.json", strict_options());
    assert!(findings.is_empty(), "Strict JSON fixture must parse.");
    let mut object = object.expect("Strict JSON fixture must produce an object.");

    assert!(!object.set_scalar(
        &["blocked", "strict"],
        ConfigScalar::Bool(true),
        NonObjectParentAction::Preserve
    ));
    assert!(!object.object_exists(&["blocked"]));
    assert_eq!(object.render(), bytes);
    assert_eq!(object.scalar(&["kept"]), Some(ConfigScalar::Bool(true)));

    assert!(object.set_scalar(
        &["blocked", "strict"],
        ConfigScalar::Bool(true),
        NonObjectParentAction::Replace
    ));
    assert_eq!(
        object.scalar(&["blocked", "strict"]),
        Some(ConfigScalar::Bool(true))
    );
}

#[test]
fn extended_numbers_do_not_enable_hexadecimal_numbers() {
    let mut options = strict_options();
    options.allow_extended_json_numbers = true;
    let (object, findings) =
        parse_object_or_report(Some(br#"{"value":0x10}"#), "test.jsonc", options);
    assert!(object.is_none());
    assert!(matches!(findings.as_slice(), [Finding::ParseError { .. }]));
}

#[test]
fn replacement_and_insertion_preserve_unmanaged_bytes_exactly() {
    let bytes = b"{\r\n  // kept\r\n  \"strict\": false,\r\n  \"nested\": {},\r\n}\r\n";
    let mut object = parse(bytes);
    assert!(object.set_scalar(
        &["strict"],
        ConfigScalar::Bool(true),
        NonObjectParentAction::Preserve
    ));
    assert!(object.set_scalar(
        &["nested", "added"],
        ConfigScalar::Int(1),
        NonObjectParentAction::Preserve
    ));
    assert_eq!(
        object.render(),
        b"{\r\n  // kept\r\n  \"strict\": true,\r\n  \"nested\": {\r\n    \"added\": 1\r\n  },\r\n}\r\n"
    );
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
fn scalar_replacement_discards_descendant_number_metadata() {
    let mut object = parse(br#"{"parent":{"value":0b1}}"#);
    assert!(object.set_scalar(
        &["parent"],
        ConfigScalar::Bool(true),
        NonObjectParentAction::Preserve
    ));
    assert_eq!(object.scalar(&["parent", "value"]), None);
    assert_eq!(object.rendered_value(&["parent", "value"]), None);
    assert_eq!(object.render(), br#"{"parent":true}"#);
}

#[test]
fn intermediate_parent_replacement_discards_parent_number_metadata() {
    let mut object = parse(br#"{"parent":0b1}"#);
    assert!(object.set_scalar(
        &["parent", "child"],
        ConfigScalar::Bool(true),
        NonObjectParentAction::Replace
    ));
    assert_eq!(
        object.rendered_value(&["parent"]),
        Some("{\n    \"child\": true\n  }".to_owned())
    );
    assert_eq!(object.render(), b"{\"parent\":{\n    \"child\": true\n  }}");
}

#[test]
fn scalar_access_supports_the_full_signed_i64_range() {
    let strict = parse_object_or_report(
        Some(br#"{"value":-9223372036854775808}"#),
        "test.json",
        strict_options(),
    );
    assert!(
        strict.1.is_empty(),
        "Signed-limit fixture must parse: {:?}",
        strict.1
    );
    assert_eq!(
        strict
            .0
            .expect("Fixture must produce an object.")
            .scalar(&["value"]),
        Some(ConfigScalar::Int(i64::MIN))
    );

    let extended = parse(br#"{"value":-0x8000000000000000}"#);
    assert_eq!(
        extended.scalar(&["value"]),
        Some(ConfigScalar::Int(i64::MIN))
    );
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
        "compiler.strict".to_owned(),
        Some("strict".to_owned()),
        NonObjectParentAction::Preserve,
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

#[test]
fn object_creation_uses_shared_parent_write_rules() {
    let mut object = parse(br#"{"existing":true}"#);
    assert!(object.set_object(&["nested", "empty"], NonObjectParentAction::Preserve));
    assert!(object.object_exists(&["nested", "empty"]));

    let mut blocked = parse(br#"{"nested":false}"#);
    assert!(!blocked.set_object(&["nested", "empty"], NonObjectParentAction::Preserve));
    assert_eq!(blocked.render(), br#"{"nested":false}"#);
}

fn parse(bytes: &[u8]) -> JsonObject {
    let (object, findings) = parse_object_or_report(Some(bytes), "test.jsonc", options());
    assert!(findings.is_empty(), "Fixture must parse: {findings:?}");
    object.expect("Fixture must produce an object.")
}

const fn strict_options() -> JsonParseOptions {
    JsonParseOptions {
        allow_comments: false,
        allow_loose_object_property_names: false,
        allow_trailing_commas: false,
        allow_missing_commas: false,
        allow_single_quoted_strings: false,
        allow_hexadecimal_numbers: false,
        allow_unary_plus_numbers: false,
        allow_extended_json_numbers: false,
        allow_extended_string_escapes: false,
        allow_extended_whitespace: false,
        allow_utf8_bom: false,
    }
}

const fn options() -> JsonParseOptions {
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
