use std::collections::{BTreeMap, BTreeSet};

use aqc_file_engine_core::{
    Finding, ItemRequirements, KeyedItem, Provenance, ResolvedItemRequirements,
    ResolvedRequirement, ScalarAssertion, resolve_items,
};
use aqc_yaml_engine_core::{
    YamlFieldError, YamlFieldValue, apply_scalar_assertion, parse_yaml_mapping,
    remove_rejected_effective_root_keys, report_missing_effective_root_keys,
};
use yaml_edit as _;

#[test]
fn missing_mapping_writes_deterministically() {
    let document =
        parse_yaml_mapping(None, "test.yaml").expect("Missing YAML must create a mapping.");
    document.set_boolean("enabled", true);
    document.set_integer("age", 1440);
    document.set_string("mode", "error");
    assert_eq!(
        String::from_utf8(document.render()).expect("Rendered YAML must be UTF-8."),
        "enabled: true\nage: 1440\nmode: error\n"
    );
}

#[test]
fn parsing_without_writes_preserves_exact_bytes() {
    let bytes = b"# comment\nenabled: true  # inline\n";
    let document = parse_yaml_mapping(Some(bytes), "test.yaml").expect("YAML must parse.");
    assert_eq!(document.render(), bytes);
}

#[test]
fn removing_the_final_key_renders_a_valid_empty_mapping() {
    let document =
        parse_yaml_mapping(Some(b"only: value\n"), "test.yaml").expect("YAML must parse.");
    document.remove("only");
    let rendered = document.render();
    assert_eq!(rendered, b"{}\n");
    let reparsed = parse_yaml_mapping(Some(&rendered), "test.yaml")
        .expect("Rendered empty mapping must parse.");
    assert!(reparsed.direct_keys().is_empty());
}

#[test]
fn duplicate_and_non_string_root_keys_fail_closed() {
    assert!(parse_yaml_mapping(Some(b"a: 1\na: 2\n"), "test.yaml").is_err());
    for bytes in [
        b"1: value\n".as_slice(),
        b"1.5: value\n",
        b"null: value\n",
        b"true: value\n",
    ] {
        assert!(parse_yaml_mapping(Some(bytes), "test.yaml").is_err());
    }
    assert!(parse_yaml_mapping(Some(b"outer:\n  a: 1\n  a: 2\n"), "test.yaml").is_err());
}

#[test]
fn non_mapping_root_fails_closed() {
    for bytes in [b"- one\n- two\n".as_slice(), b"scalar\n", b"null\n"] {
        assert!(parse_yaml_mapping(Some(bytes), "test.yaml").is_err());
    }
}

#[test]
fn malformed_yaml_fails_closed() {
    assert!(parse_yaml_mapping(Some(b"value: [unterminated\n"), "test.yaml").is_err());
}

#[test]
fn yaml_11_boolean_words_and_quoted_bools_are_strings() {
    for text in ["yes", "no", "on", "off", "'true'", "\"false\""] {
        let bytes = format!("value: {text}\n");
        let document =
            parse_yaml_mapping(Some(bytes.as_bytes()), "test.yaml").expect("YAML must parse.");
        assert!(matches!(
            document.field("value"),
            Ok(Some(YamlFieldValue::String(_)))
        ));
    }
    for (text, expected) in [("true", true), ("FALSE", false)] {
        let bytes = format!("value: {text}\n");
        let document =
            parse_yaml_mapping(Some(bytes.as_bytes()), "test.yaml").expect("YAML must parse.");
        assert_eq!(
            document.field("value"),
            Ok(Some(YamlFieldValue::Boolean(expected)))
        );
    }
}

#[test]
fn collection_string_positions_reject_yaml_12_non_strings() {
    for value in ["1", "1.5", "null", "true"] {
        let sequence = format!("value: [{value}]\n");
        let sequence_document =
            parse_yaml_mapping(Some(sequence.as_bytes()), "test.yaml").expect("YAML must parse.");
        assert_eq!(
            sequence_document.field("value"),
            Err(aqc_yaml_engine_core::YamlFieldError::WrongShape)
        );

        let mapping = format!("value:\n  {value}: true\n");
        let mapping_document =
            parse_yaml_mapping(Some(mapping.as_bytes()), "test.yaml").expect("YAML must parse.");
        assert_eq!(
            mapping_document.field("value"),
            Err(aqc_yaml_engine_core::YamlFieldError::WrongShape)
        );
    }
}

#[test]
fn direct_and_effective_root_keys_are_distinct() {
    let document = parse_yaml_mapping(
        Some(b"defaults: &defaults\n  inherited: true\n<<: *defaults\ndirect: true\n"),
        "test.yaml",
    )
    .expect("YAML must parse.");

    assert_eq!(document.direct_keys(), vec!["defaults", "<<", "direct"]);
    assert_eq!(
        document.effective_keys(),
        Ok(vec![
            "defaults".to_owned(),
            "direct".to_owned(),
            "inherited".to_owned()
        ])
    );
}

#[test]
fn merged_lookup_uses_direct_precedence_and_resolves_aliases() {
    let bytes =
        b"defaults: &defaults\n  enabled: false\n  mode: error\n<<: *defaults\nenabled: true\n";
    let document = parse_yaml_mapping(Some(bytes), "test.yaml").expect("YAML must parse.");
    assert_eq!(
        document.field("enabled"),
        Ok(Some(YamlFieldValue::Boolean(true)))
    );
    assert_eq!(
        document.field("mode"),
        Ok(Some(YamlFieldValue::String("error".to_owned())))
    );
}

#[test]
fn merge_sequences_use_yaml_precedence() {
    let bytes = b"first: &first\n  shared: first\n  one: 1\nsecond: &second\n  shared: second\n  two: 2\n<<: [*first, *second]\n";
    let document = parse_yaml_mapping(Some(bytes), "test.yaml").expect("YAML must parse.");
    assert_eq!(
        document.field("shared"),
        Ok(Some(YamlFieldValue::String("first".to_owned())))
    );
    assert_eq!(document.field("one"), Ok(Some(YamlFieldValue::Integer(1))));
    assert_eq!(document.field("two"), Ok(Some(YamlFieldValue::Integer(2))));
}

#[test]
fn mapping_valued_merge_sources_are_effective() {
    let document = parse_yaml_mapping(Some(b"<<: {enabled: true, mode: error}\n"), "test.yaml")
        .expect("YAML must parse.");
    assert_eq!(
        document.field("enabled"),
        Ok(Some(YamlFieldValue::Boolean(true)))
    );
    assert_eq!(
        document.field("mode"),
        Ok(Some(YamlFieldValue::String("error".to_owned())))
    );
}

#[test]
fn nested_mappings_resolve_merge_sources() {
    let document = parse_yaml_mapping(
        Some(
            b"buildDefaults: &buildDefaults\n  esbuild: true\nallowBuilds:\n  <<: *buildDefaults\n  blocked: false\n",
        ),
        "test.yaml",
    )
    .expect("YAML must parse.");
    assert_eq!(
        document.field("allowBuilds"),
        Ok(Some(YamlFieldValue::StringBooleanMapping(BTreeMap::from(
            [("blocked".to_owned(), false), ("esbuild".to_owned(), true),]
        ))))
    );
}

#[test]
fn quoted_merge_key_is_an_ordinary_key() {
    let document = parse_yaml_mapping(Some(b"\"<<\": ordinary\nenabled: true\n"), "test.yaml")
        .expect("Quoted merge key must parse as an ordinary string key.");
    assert_eq!(
        document.field("<<"),
        Ok(Some(YamlFieldValue::String("ordinary".to_owned())))
    );
    assert_eq!(
        document.effective_keys(),
        Ok(vec!["<<".to_owned(), "enabled".to_owned()])
    );
}

#[test]
fn quoted_merge_key_never_injects_effective_fields() {
    let document = parse_yaml_mapping(
        Some(b"\"<<\": {injected: true}\nenabled: false\n"),
        "test.yaml",
    )
    .expect("Quoted merge key must remain ordinary data.");
    assert_eq!(
        document.effective_keys(),
        Ok(vec!["<<".to_owned(), "enabled".to_owned()])
    );
    assert_eq!(document.field("injected"), Ok(None));
}

#[test]
fn tagged_and_aliased_string_mapping_keys_decode() {
    let tagged = parse_yaml_mapping(Some(b"? !!str enabled\n: true\n"), "test.yaml")
        .expect("A tagged string key must parse.");
    assert_eq!(
        tagged.field("enabled"),
        Ok(Some(YamlFieldValue::Boolean(true)))
    );

    let aliased = parse_yaml_mapping(
        Some(b"keyName: &keyName enabled\n? *keyName\n: true\n"),
        "test.yaml",
    )
    .expect("An aliased string key must parse.");
    assert_eq!(
        aliased.field("enabled"),
        Ok(Some(YamlFieldValue::Boolean(true)))
    );
}

#[test]
fn differently_typed_nested_keys_are_not_duplicate_syntax() {
    let document = parse_yaml_mapping(
        Some(b"allowBuilds: {true: false, \"true\": false}\n"),
        "test.yaml",
    )
    .expect("Distinct typed YAML keys must survive syntax validation.");
    assert_eq!(
        document.field("allowBuilds"),
        Err(YamlFieldError::WrongShape)
    );
}

#[test]
fn direct_write_overrides_merge_without_rewriting_anchor() {
    let bytes = b"defaults: &defaults\n  enabled: false\n<<: *defaults\n";
    let document = parse_yaml_mapping(Some(bytes), "test.yaml").expect("YAML must parse.");
    document.set_boolean("enabled", true);
    let rendered = String::from_utf8(document.render()).expect("Rendered YAML must be UTF-8.");
    assert!(rendered.contains("defaults: &defaults\n  enabled: false\n"));
    assert!(rendered.contains("<<: *defaults\n"));
    assert!(rendered.ends_with("enabled: true\n"));
}

#[test]
fn unresolved_alias_and_invalid_merge_source_are_field_errors() {
    let alias =
        parse_yaml_mapping(Some(b"value: *missing\n"), "test.yaml").expect("YAML must parse.");
    assert_eq!(alias.field("value"), Err(YamlFieldError::UnresolvedAlias));
    let merge =
        parse_yaml_mapping(Some(b"<<: [not-an-alias]\n"), "test.yaml").expect("YAML must parse.");
    assert_eq!(
        merge.field("value"),
        Err(YamlFieldError::InvalidMergeSource)
    );
    let missing_merge =
        parse_yaml_mapping(Some(b"<<: *missing\n"), "test.yaml").expect("YAML syntax must parse.");
    assert_eq!(
        missing_merge.field("value"),
        Err(YamlFieldError::UnresolvedAlias)
    );
    let direct = parse_yaml_mapping(Some(b"<<: *missing\nvalue: direct\n"), "test.yaml")
        .expect("YAML syntax must parse.");
    assert_eq!(
        direct.field("value"),
        Ok(Some(YamlFieldValue::String("direct".to_owned())))
    );
}

#[test]
fn cyclic_merge_alias_is_an_invalid_merge_source() {
    let document = parse_yaml_mapping(
        Some(b"defaults: &defaults\n  <<: *defaults\n<<: *defaults\n"),
        "test.yaml",
    )
    .expect("Cyclic aliases are valid YAML syntax.");
    assert_eq!(
        document.field("missing"),
        Err(YamlFieldError::InvalidMergeSource)
    );
    assert_eq!(
        document.effective_keys(),
        Err(YamlFieldError::InvalidMergeSource)
    );
}

#[test]
fn standard_tags_decode_and_unknown_tags_fail() {
    for (yaml, expected) in [
        ("value: !!bool true\n", YamlFieldValue::Boolean(true)),
        (
            "value: !!str true\n",
            YamlFieldValue::String("true".to_owned()),
        ),
        ("value: !!int 1440\n", YamlFieldValue::Integer(1440)),
    ] {
        let document =
            parse_yaml_mapping(Some(yaml.as_bytes()), "test.yaml").expect("YAML must parse.");
        assert_eq!(document.field("value"), Ok(Some(expected)));
    }
    let unknown =
        parse_yaml_mapping(Some(b"value: !custom true\n"), "test.yaml").expect("YAML must parse.");
    assert_eq!(unknown.field("value"), Err(YamlFieldError::UnknownTag));
}

#[test]
fn direct_collection_writes_round_trip() {
    let document =
        parse_yaml_mapping(None, "test.yaml").expect("Missing YAML must create a mapping.");
    document.set_string_sequence("packages", &["a".to_owned(), "@scope/b".to_owned()]);
    let values = BTreeMap::from([("a".to_owned(), true), ("b".to_owned(), false)]);
    document.set_string_boolean_mapping("allowBuilds", &values);
    let rendered = document.render();
    let reparsed =
        parse_yaml_mapping(Some(&rendered), "test.yaml").expect("Rendered YAML must parse.");
    assert_eq!(
        reparsed.field("packages"),
        Ok(Some(YamlFieldValue::StringSequence(vec![
            "a".to_owned(),
            "@scope/b".to_owned()
        ])))
    );
    assert_eq!(
        reparsed.field("allowBuilds"),
        Ok(Some(YamlFieldValue::StringBooleanMapping(values)))
    );
}

#[test]
fn empty_collection_writes_round_trip() {
    let document = parse_yaml_mapping(
        Some(b"packages:\n- inherited\nallowBuilds: {inherited: true}\n"),
        "test.yaml",
    )
    .expect("YAML must parse.");
    document.set_string_sequence("packages", &[]);
    document.set_string_boolean_mapping("allowBuilds", &BTreeMap::new());
    let rendered = document.render();
    let reparsed = parse_yaml_mapping(Some(&rendered), "test.yaml")
        .expect("Rendered empty collections must parse.");
    assert_eq!(
        reparsed.field("packages"),
        Ok(Some(YamlFieldValue::StringSequence(Vec::new())))
    );
    assert_eq!(
        reparsed.field("allowBuilds"),
        Ok(Some(YamlFieldValue::StringBooleanMapping(BTreeMap::new())))
    );
}

#[test]
fn aliases_inside_managed_collections_resolve_to_typed_values() {
    let document = parse_yaml_mapping(
        Some(
            b"package: &package react\nenabled: &enabled true\npackages: [*package]\nallowBuilds:\n  react: *enabled\n",
        ),
        "test.yaml",
    )
    .expect("YAML must parse.");
    assert_eq!(
        document.field("packages"),
        Ok(Some(YamlFieldValue::StringSequence(vec![
            "react".to_owned()
        ])))
    );
    assert_eq!(
        document.field("allowBuilds"),
        Ok(Some(YamlFieldValue::StringBooleanMapping(BTreeMap::from(
            [("react".to_owned(), true,)]
        ))))
    );
}

#[test]
fn generic_scalar_reconciliation_reads_writes_and_attributes() {
    let provenance = Provenance {
        policy: "policy".to_owned(),
    };
    let assertion = ScalarAssertion::Equals(true, "must be enabled".to_owned());
    let resolved = ResolvedRequirement {
        merged: assertion.clone(),
        collected: vec![(provenance.clone(), assertion)],
    };
    let document =
        parse_yaml_mapping(Some(b"enabled: false\n"), "test.yaml").expect("YAML must parse.");
    let mut findings = Vec::new();
    apply_scalar_assertion(&document, "enabled", Some(&resolved), &mut findings);
    assert!(matches!(
        findings.as_slice(),
        [Finding::Mismatch { attribution, .. }] if attribution == &[provenance]
    ));
    assert_eq!(
        document.field("enabled"),
        Ok(Some(YamlFieldValue::Boolean(true)))
    );
}

#[test]
fn parent_removal_precedes_child_reconciliation() {
    let requirements = resolved_root_keys(ItemRequirements {
        required: vec![(root_key("missing"), "missing key".to_owned())],
        forbidden: vec![(root_key("forbidden"), "forbidden key".to_owned())],
        exact: Some((
            vec![root_key("allowed"), root_key("missing")],
            "exact keys".to_owned(),
        )),
    });
    let document = parse_yaml_mapping(
        Some(b"allowed: true\nforbidden: false\nunexpected: value\n"),
        "test.yaml",
    )
    .expect("YAML must parse.");
    let mut findings = Vec::new();

    let rejected = remove_rejected_effective_root_keys(&document, &requirements, &mut findings)
        .expect("effective keys must resolve");
    report_missing_effective_root_keys(&document, &requirements, &mut findings);

    assert_eq!(document.render(), b"allowed: true");
    assert_eq!(
        rejected,
        BTreeSet::from(["forbidden".to_owned(), "unexpected".to_owned()])
    );
    assert!(
        findings.iter().any(|finding| matches!(
            finding,
            Finding::UnwritableRequiredKey { key, attribution, .. }
                if key == "missing"
                    && attribution.len() == 2
                    && attribution.iter().all(|item| item == &test_provenance())
        )),
        "findings: {findings:?}"
    );
    assert!(findings.iter().any(|finding| matches!(
        finding,
        Finding::Mismatch { key, message, .. }
            if key == "forbidden" && message == "forbidden key"
    )));
    assert!(findings.iter().any(|finding| matches!(
        finding,
        Finding::Mismatch { key, message, .. }
            if key == "unexpected" && message == "exact keys"
    )));
}

#[test]
fn inherited_extra_is_reported_without_rewriting_anchor_sources() {
    let bytes = b"defaults: &defaults\n  inherited: true\n<<: *defaults\n";
    let document = parse_yaml_mapping(Some(bytes), "test.yaml").expect("YAML must parse.");
    let requirements = resolved_root_keys(ItemRequirements {
        exact: Some((Vec::new(), "no effective keys".to_owned())),
        ..ItemRequirements::default()
    });
    let mut findings = Vec::new();

    let rejected = remove_rejected_effective_root_keys(&document, &requirements, &mut findings)
        .expect("effective keys must resolve");
    report_missing_effective_root_keys(&document, &requirements, &mut findings);

    assert_eq!(document.render(), bytes);
    assert_eq!(
        rejected,
        BTreeSet::from(["defaults".to_owned(), "inherited".to_owned()])
    );
    assert!(findings.iter().any(|finding| matches!(
        finding,
        Finding::Mismatch { key, .. } if key == "inherited"
    )));
    assert!(!findings.iter().any(|finding| matches!(
        finding,
        Finding::Mismatch { key, .. } if key == "<<"
    )));
}

#[test]
fn direct_key_removal_preserves_anchors_used_by_retained_keys() {
    let bytes = b"shared: &value true\nstrictPeerDependencies: *value\n";
    let document = parse_yaml_mapping(Some(bytes), "test.yaml").expect("YAML must parse.");

    assert!(!document.remove_if_effectively_absent("shared"));
    assert_eq!(document.render(), bytes);
    assert_eq!(
        document.field("strictPeerDependencies"),
        Ok(Some(YamlFieldValue::Boolean(true)))
    );
}

fn resolved_root_keys(
    requirements: ItemRequirements<KeyedItem<()>>,
) -> ResolvedItemRequirements<KeyedItem<()>> {
    let mut conflicts = Vec::new();
    let resolved = resolve_items(
        "test.yaml",
        vec![(test_provenance(), requirements)],
        &mut conflicts,
    );
    assert!(conflicts.is_empty(), "root-key fixture must resolve");
    resolved
}

fn root_key(file_key: &str) -> KeyedItem<()> {
    KeyedItem {
        file_key: file_key.to_owned(),
        value: (),
    }
}

fn test_provenance() -> Provenance {
    Provenance {
        policy: "policy".to_owned(),
    }
}
