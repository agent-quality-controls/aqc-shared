#![allow(
    clippy::expect_used,
    reason = "Tests use expect to fail loudly when fixture invariants are broken."
)]
use std::collections::{BTreeMap, BTreeSet};

use aqc_file_engine_core::{
    ConfigScalar, FileItemRequirement, Finding, ItemAssertionInput, ItemRequirements, KeyedItem,
    ListRequirements, Provenance, RequiredItemResolution, ScalarAssertion, compose_item_by,
    resolve_items, resolve_list,
};
use aqc_toml_engine_core::{
    ListFieldKeyStyle, ScalarFieldEdit, TomlArrayItem, TomlArrayTableItem, TomlItemError,
    TomlItemField, parse_or_report, reconcile_array_items, reconcile_array_table_items,
    reconcile_list_field, reconcile_optional_list_field, remove_rejected_table_keys,
    report_list_shape, report_missing_table_keys, scalar_field_edit, table_at, table_list_values,
    write_table_list,
};
use toml_edit::{DocumentMut, Table, TableLike, Value};

type MismatchKeyAndExpected<'a> = (&'a str, &'a str);

#[derive(Debug, Clone, PartialEq, Eq)]
struct TestItem {
    key: String,
    value: String,
}

impl FileItemRequirement for TestItem {
    type Identity = String;

    fn merge_identity(&self) -> Self::Identity {
        self.key.clone()
    }

    fn compose_item(
        key: &str,
        items: Vec<ItemAssertionInput<Self>>,
        conflicts: &mut Vec<aqc_file_engine_core::ConflictEntry>,
    ) -> Option<RequiredItemResolution<Self>> {
        compose_item_by(key, items, |item| item.value.clone(), conflicts)
    }
}

impl TomlArrayItem for TestItem {
    fn read_value(value: &Value) -> Result<Self, TomlItemError> {
        let text = value
            .as_str()
            .ok_or_else(|| TomlItemError::new("expected string"))?;
        let (key, item_value) = text
            .split_once('=')
            .ok_or_else(|| TomlItemError::new("expected key=value"))?;
        Ok(Self {
            key: key.to_owned(),
            value: item_value.to_owned(),
        })
    }

    fn write_value(&self) -> Value {
        Value::from(format!("{}={}", self.key, self.value))
    }

    fn matches_value(current: &Self, required: &Self) -> bool {
        current == required
    }

    fn render_value(&self) -> String {
        format!("{}={}", self.key, self.value)
    }
}

impl TomlArrayTableItem for TestItem {
    fn read_table(table: &dyn TableLike) -> Result<Self, TomlItemError> {
        let key = table
            .get("key")
            .and_then(toml_edit::Item::as_str)
            .ok_or_else(|| TomlItemError::new("missing key"))?;
        let value = table
            .get("value")
            .and_then(toml_edit::Item::as_str)
            .ok_or_else(|| TomlItemError::new("missing value"))?;
        Ok(Self {
            key: key.to_owned(),
            value: value.to_owned(),
        })
    }

    fn write_table(&self) -> Table {
        let mut table = Table::new();
        drop(table.insert("key", toml_edit::value(&self.key)));
        drop(table.insert("value", toml_edit::value(&self.value)));
        table
    }

    fn matches_table(current: &Self, required: &Self) -> bool {
        current == required
    }

    fn render_table(&self) -> String {
        format!("{}={}", self.key, self.value)
    }
}

fn provenance() -> Provenance {
    provenance_named("policy")
}

fn provenance_named(policy: &str) -> Provenance {
    Provenance {
        policy: policy.to_owned(),
    }
}

fn parse_error_message(finding: &Finding) -> Option<&str> {
    match finding {
        Finding::ParseError { message, .. } => Some(message),
        Finding::Mismatch { .. }
        | Finding::UnwritableRequiredKey { .. }
        | Finding::InvalidRequirements { .. }
        | Finding::ConflictingRequirements { .. }
        | Finding::InternalError { .. } => None,
    }
}

fn mismatch_key_and_expected(finding: &Finding) -> Option<MismatchKeyAndExpected<'_>> {
    match finding {
        Finding::Mismatch { key, expected, .. } => Some((key, expected)),
        Finding::UnwritableRequiredKey { .. }
        | Finding::InvalidRequirements { .. }
        | Finding::ParseError { .. }
        | Finding::ConflictingRequirements { .. }
        | Finding::InternalError { .. } => None,
    }
}

fn exact_test_requirements() -> aqc_file_engine_core::ResolvedItemRequirements<TestItem> {
    let mut conflicts = Vec::new();
    let resolved = resolve_items(
        "items",
        vec![(
            provenance(),
            ItemRequirements {
                required: Vec::new(),
                forbidden: Vec::new(),
                exact: Some((
                    vec![TestItem {
                        key: "a".to_owned(),
                        value: "1".to_owned(),
                    }],
                    "exact item".to_owned(),
                )),
            },
        )],
        &mut conflicts,
    );
    assert!(conflicts.is_empty(), "exact requirements must resolve");
    resolved
}

fn resolved_table_keys(
    requirements: ItemRequirements<KeyedItem<()>>,
) -> aqc_file_engine_core::ResolvedItemRequirements<KeyedItem<()>> {
    let mut conflicts = Vec::new();
    let resolved = resolve_items("table", vec![(provenance(), requirements)], &mut conflicts);
    assert!(conflicts.is_empty(), "table-key requirements must resolve");
    resolved
}

fn table_key(key: &str) -> KeyedItem<()> {
    KeyedItem {
        file_key: key.to_owned(),
        value: (),
    }
}

#[test]
fn exact_array_items_create_missing_members_and_repair_values() {
    let requirements = exact_test_requirements();
    let field = TomlItemField::new(&[], "items", "items");
    let mut missing = DocumentMut::new();
    let mut findings = Vec::new();
    reconcile_array_items(&mut missing, field, &requirements, &mut findings);
    assert_eq!(missing.to_string(), "items = [\"a=1\"]\n");
    assert_eq!(findings.len(), 1);

    let mut wrong = "items = [\"a=2\"]\n"
        .parse::<DocumentMut>()
        .expect("the list-field test fixture must parse as valid TOML");
    findings.clear();
    reconcile_array_items(&mut wrong, field, &requirements, &mut findings);
    assert_eq!(wrong.to_string(), "items = [\"a=1\"]\n");
    assert_eq!(findings.len(), 1);
}

#[test]
fn exact_array_table_items_create_missing_members_and_repair_values() {
    let requirements = exact_test_requirements();
    let field = TomlItemField::new(&[], "items", "items");
    let mut missing = DocumentMut::new();
    let mut findings = Vec::new();
    reconcile_array_table_items(&mut missing, field, &requirements, &mut findings);
    assert!(missing.to_string().contains("value = \"1\""));
    assert_eq!(findings.len(), 1);

    let mut wrong = "[[items]]\nkey = \"a\"\nvalue = \"2\"\n"
        .parse::<DocumentMut>()
        .expect("the array-table test fixture must parse as valid TOML");
    findings.clear();
    reconcile_array_table_items(&mut wrong, field, &requirements, &mut findings);
    assert!(wrong.to_string().contains("value = \"1\""));
    assert_eq!(findings.len(), 1);
}

#[test]
fn table_key_reconciliation_reports_missing_required_keys_as_unwritable() {
    let requirements = resolved_table_keys(ItemRequirements {
        required: vec![(table_key("required"), "required message".to_owned())],
        ..ItemRequirements::default()
    });
    let mut table = Table::new();
    let mut findings = Vec::new();

    remove_rejected_table_keys(&mut table, "settings", &requirements, &mut findings);
    report_missing_table_keys(&table, "settings", &requirements, &mut findings);

    assert!(table.is_empty());
    assert!(matches!(
        findings.as_slice(),
        [Finding::UnwritableRequiredKey { key, expected, attribution }]
            if key == "settings.required"
                && expected == "present table key"
                && attribution == &[provenance()]
    ));
}

#[test]
fn child_reconciliation_satisfies_membership_without_a_duplicate_missing_finding() {
    let requirements = resolved_table_keys(ItemRequirements {
        exact: Some((vec![table_key("required")], "exact message".to_owned())),
        ..ItemRequirements::default()
    });
    let mut table = Table::new();
    let mut findings = Vec::new();

    remove_rejected_table_keys(&mut table, "settings", &requirements, &mut findings);
    let _ = table.insert("required", toml_edit::value(true));
    findings.push(Finding::Mismatch {
        key: "settings.required".to_owned(),
        selector: None,
        current: None,
        expected: "equals true".to_owned(),
        message: "child value".to_owned(),
        severity: aqc_file_engine_core::Severity::Error,
        attribution: vec![provenance()],
    });
    report_missing_table_keys(&table, "settings", &requirements, &mut findings);

    assert_eq!(findings.len(), 1);
}

#[test]
fn table_key_reconciliation_removes_forbidden_and_unexpected_keys() {
    let requirements = resolved_table_keys(ItemRequirements {
        forbidden: vec![(table_key("forbidden"), "forbidden message".to_owned())],
        exact: Some((vec![table_key("allowed")], "exact message".to_owned())),
        ..ItemRequirements::default()
    });
    let mut table = Table::new();
    let _ = table.insert("allowed", toml_edit::value(true));
    let _ = table.insert("forbidden", toml_edit::value(false));
    let _ = table.insert("unexpected", toml_edit::value(1));
    let mut findings = Vec::new();

    remove_rejected_table_keys(&mut table, "settings", &requirements, &mut findings);
    report_missing_table_keys(&table, "settings", &requirements, &mut findings);

    assert!(table.contains_key("allowed"));
    assert!(!table.contains_key("forbidden"));
    assert!(!table.contains_key("unexpected"));
    assert!(findings.iter().any(|finding| matches!(
        finding,
        Finding::Mismatch { key, expected, message, .. }
            if key == "settings.forbidden"
                && expected == "absent"
                && message == "forbidden message"
    )));
    assert!(findings.iter().any(|finding| matches!(
        finding,
        Finding::Mismatch { key, expected, message, .. }
            if key == "settings.unexpected"
                && expected == "absent (exact keys)"
                && message == "exact message"
    )));
}

#[test]
fn exact_empty_table_key_reconciliation_removes_every_key() {
    let requirements = resolved_table_keys(ItemRequirements {
        exact: Some((Vec::new(), "exact empty".to_owned())),
        ..ItemRequirements::default()
    });
    let mut table = Table::new();
    let _ = table.insert("one", toml_edit::value(1));
    let _ = table.insert("two", toml_edit::value(2));
    let mut findings = Vec::new();

    remove_rejected_table_keys(&mut table, "", &requirements, &mut findings);
    report_missing_table_keys(&table, "", &requirements, &mut findings);

    assert!(table.is_empty());
    assert_eq!(findings.len(), 2);
    assert!(findings.iter().all(|finding| matches!(
        finding,
        Finding::Mismatch { key, attribution, .. }
            if (key == "one" || key == "two") && attribution == &[provenance()]
    )));
}

#[test]
fn parse_or_report_reports_invalid_utf8() {
    let (_, findings) = parse_or_report(Some(&[0xff]), "config.toml");

    assert_eq!(findings.len(), 1);
    let message = parse_error_message(findings.first().expect("one parse error"))
        .expect("finding is a parse error");
    assert!(message.contains("not valid UTF-8"));
}

#[test]
fn parse_or_report_reports_invalid_toml() {
    let (_, findings) = parse_or_report(Some(b"key = [\n"), "config.toml");

    assert_eq!(findings.len(), 1);
    let message = parse_error_message(findings.first().expect("one parse error"))
        .expect("finding is a parse error");
    assert!(message.contains("not valid TOML"));
}

#[test]
fn scalar_helpers_render_and_match_string_bool_and_int() {
    let cases = [
        (ConfigScalar::Str("value".to_owned()), "\"value\""),
        (ConfigScalar::Bool(true), "true"),
        (ConfigScalar::Int(42), "42"),
    ];

    for (scalar, rendered_item) in cases {
        let item = aqc_toml_engine_core::scalar_item(&scalar);
        assert!(aqc_toml_engine_core::scalar_matches(&item, &scalar));
        assert_eq!(
            aqc_toml_engine_core::render_scalar(&scalar),
            rendered_item.trim_matches('"')
        );
        assert_eq!(
            aqc_toml_engine_core::render_item(&item),
            Some(rendered_item.to_owned())
        );
    }
}

#[test]
fn scalar_assertion_fails_matches_core_scalar_verbs() {
    let item = aqc_toml_engine_core::scalar_item(&ConfigScalar::Str("2024".to_owned()));
    let mut allowed = BTreeSet::new();
    let _ = allowed.insert(ConfigScalar::Str("2021".to_owned()));

    assert!(!aqc_toml_engine_core::scalar_assertion_fails(
        Some(&item),
        &ScalarAssertion::Equals(ConfigScalar::Str("2024".to_owned()), "msg".to_owned())
    ));
    assert!(aqc_toml_engine_core::scalar_assertion_fails(
        Some(&item),
        &ScalarAssertion::OneOf(allowed, "msg".to_owned())
    ));
    assert!(!aqc_toml_engine_core::scalar_assertion_fails(
        Some(&item),
        &ScalarAssertion::Present("msg".to_owned())
    ));
    assert!(aqc_toml_engine_core::scalar_assertion_fails(
        Some(&item),
        &ScalarAssertion::Absent("msg".to_owned())
    ));
}

#[test]
fn present_requires_a_typed_scalar_and_absent_removes_any_shape() {
    let table = "[setting]\nvalue = true\n"
        .parse::<toml_edit::DocumentMut>()
        .expect("The rendered scalar fixture must remain valid TOML.");
    let item = table
        .get("setting")
        .expect("The rendered TOML table must contain the setting key.");
    assert!(aqc_toml_engine_core::scalar_assertion_fails(
        Some(item),
        &ScalarAssertion::Present("scalar required".to_owned())
    ));

    let mut findings = Vec::new();
    assert!(matches!(
        scalar_field_edit(
            "setting".to_owned(),
            Some(item),
            &ScalarAssertion::Absent("remove".to_owned()),
            &[provenance()],
            &mut findings,
        ),
        Some(ScalarFieldEdit::Remove)
    ));
    assert_eq!(findings.len(), 1);
}

#[test]
fn scalar_field_edit_reports_and_returns_write_remove_actions() {
    let item = aqc_toml_engine_core::scalar_item(&ConfigScalar::Str("old".to_owned()));
    let mut findings = Vec::new();

    let write_edit = scalar_field_edit(
        "setting".to_owned(),
        Some(&item),
        &ScalarAssertion::Equals(ConfigScalar::Str("new".to_owned()), "replace".to_owned()),
        &[provenance()],
        &mut findings,
    );
    assert!(matches!(write_edit, Some(ScalarFieldEdit::Write(_))));
    assert_eq!(findings.len(), 1);

    findings.clear();
    let remove_edit = scalar_field_edit(
        "setting".to_owned(),
        Some(&item),
        &ScalarAssertion::Absent("remove".to_owned()),
        &[provenance()],
        &mut findings,
    );
    assert!(matches!(remove_edit, Some(ScalarFieldEdit::Remove)));
    assert_eq!(findings.len(), 1);
}

#[test]
fn table_helpers_read_nested_standard_tables() {
    let (doc, findings) = parse_or_report(
        Some(b"[workspace.package]\nedition = \"2024\"\n"),
        "Cargo.toml",
    );
    assert!(findings.is_empty());

    let path = ["workspace".to_owned(), "package".to_owned()];
    let table = table_at(&doc, &path);

    assert!(table.is_some_and(|table| table.contains_key("edition")));
}

#[test]
fn table_helpers_read_nested_inline_tables() {
    let (doc, findings) = parse_or_report(
        Some(b"workspace = { package = { edition = \"2024\" } }\n"),
        "Cargo.toml",
    );
    assert!(findings.is_empty());

    let path = ["workspace".to_owned(), "package".to_owned()];
    let table = table_at(&doc, &path);

    assert!(table.is_some_and(|table| table.contains_key("edition")));
}

#[test]
fn table_list_helpers_read_and_write_string_arrays() {
    let (mut doc, findings) =
        parse_or_report(Some(b"[package]\nkeywords = [\"aqc\", 5]\n"), "Cargo.toml");
    assert!(findings.is_empty());

    let values = table_at(&doc, &["package".to_owned()])
        .map_or_else(Vec::new, |table| table_list_values(table, "keywords"));
    assert_eq!(values, vec!["aqc".to_owned()]);

    if let Some(table) = doc
        .get_mut("package")
        .and_then(toml_edit::Item::as_table_mut)
    {
        write_table_list(
            table,
            "keywords",
            &["guardrail".to_owned(), "rust".to_owned()],
        );
    }
    let updated = table_at(&doc, &["package".to_owned()])
        .map_or_else(Vec::new, |table| table_list_values(table, "keywords"));
    assert_eq!(updated, vec!["guardrail".to_owned(), "rust".to_owned()]);
}

#[test]
fn reconcile_list_field_applies_contains_excludes_and_exact() {
    let mut requirements = ListRequirements {
        contains: BTreeMap::from([("new".to_owned(), "contains".to_owned())]),
        excludes: BTreeMap::from([("old".to_owned(), "excludes".to_owned())]),
        exact: None,
    };
    let mut conflicts = Vec::new();
    let resolved = resolve_list(
        "ignore",
        vec![(provenance(), requirements.clone())],
        &mut conflicts,
    );
    let mut findings = Vec::new();

    let updated = reconcile_list_field(
        "ignore".to_owned(),
        vec!["old".to_owned(), "kept".to_owned()],
        &resolved,
        ListFieldKeyStyle::FieldItem,
        &mut findings,
    );

    assert_eq!(updated, Some(vec!["kept".to_owned(), "new".to_owned()]));
    assert_eq!(findings.len(), 2);
    assert!(findings.iter().all(|finding| matches!(
        finding,
        Finding::Mismatch {
            selector: Some(_),
            ..
        }
    )));

    requirements.contains.clear();
    requirements.excludes.clear();
    requirements.exact = Some((vec!["only".to_owned()], "exact".to_owned()));
    let resolved_exact = resolve_list("ignore", vec![(provenance(), requirements)], &mut conflicts);
    let updated_exact = reconcile_list_field(
        "ignore".to_owned(),
        vec!["other".to_owned()],
        &resolved_exact,
        ListFieldKeyStyle::Field,
        &mut findings,
    );

    assert_eq!(updated_exact, Some(vec!["only".to_owned()]));
}

#[test]
fn exact_list_findings_are_member_specific_order_aware_and_presence_aware() {
    let requirements = ListRequirements {
        exact: Some((
            vec!["a".to_owned(), "a".to_owned(), "b".to_owned()],
            "exact".to_owned(),
        )),
        ..ListRequirements::default()
    };
    let mut conflicts = Vec::new();
    let resolved = resolve_list("values", vec![(provenance(), requirements)], &mut conflicts);
    assert!(conflicts.is_empty());

    let mut membership_findings = Vec::new();
    let membership = reconcile_list_field(
        "values".to_owned(),
        vec!["a".to_owned(), "c".to_owned()],
        &resolved,
        ListFieldKeyStyle::Field,
        &mut membership_findings,
    );
    assert_eq!(
        membership,
        Some(vec!["a".to_owned(), "a".to_owned(), "b".to_owned()])
    );
    let selectors = membership_findings
        .iter()
        .filter_map(|finding| match finding {
            Finding::Mismatch { selector, .. } => selector.clone(),
            Finding::UnwritableRequiredKey { .. }
            | Finding::InvalidRequirements { .. }
            | Finding::ParseError { .. }
            | Finding::ConflictingRequirements { .. }
            | Finding::InternalError { .. } => None,
        })
        .collect::<BTreeSet<_>>();
    assert_eq!(
        selectors,
        BTreeSet::from(["a".to_owned(), "b".to_owned(), "c".to_owned()])
    );

    let mut order_findings = Vec::new();
    let _ = reconcile_list_field(
        "values".to_owned(),
        vec!["b".to_owned(), "a".to_owned(), "a".to_owned()],
        &resolved,
        ListFieldKeyStyle::Field,
        &mut order_findings,
    );
    assert!(matches!(
        order_findings.as_slice(),
        [Finding::Mismatch { selector: None, .. }]
    ));

    let empty_requirements = ListRequirements {
        exact: Some((Vec::new(), "empty".to_owned())),
        ..ListRequirements::default()
    };
    let empty_resolved = resolve_list(
        "values",
        vec![(provenance(), empty_requirements)],
        &mut conflicts,
    );
    let mut missing_findings = Vec::new();
    let missing = reconcile_optional_list_field(
        "values".to_owned(),
        None,
        &empty_resolved,
        ListFieldKeyStyle::Field,
        &mut missing_findings,
    );
    assert_eq!(missing, Some(Vec::new()));
    assert!(matches!(
        missing_findings.as_slice(),
        [Finding::Mismatch {
            selector: None,
            current: None,
            ..
        }]
    ));
}

#[test]
fn compatible_exact_member_assertions_share_toml_member_identity() {
    let exact = ListRequirements {
        exact: Some((vec!["react".to_owned()], "exact".to_owned())),
        ..ListRequirements::default()
    };
    let contains = ListRequirements {
        contains: BTreeMap::from([("react".to_owned(), "contains".to_owned())]),
        ..ListRequirements::default()
    };
    let excludes = ListRequirements {
        excludes: BTreeMap::from([("blocked".to_owned(), "excludes".to_owned())]),
        ..ListRequirements::default()
    };
    let mut conflicts = Vec::new();
    let resolved = resolve_list(
        "values",
        vec![
            (provenance_named("exact-policy"), exact),
            (provenance_named("contains-policy"), contains),
            (provenance_named("excludes-policy"), excludes),
        ],
        &mut conflicts,
    );
    assert!(conflicts.is_empty());
    let mut findings = Vec::new();
    let _ = reconcile_list_field(
        "values".to_owned(),
        Vec::new(),
        &resolved,
        ListFieldKeyStyle::FieldItem,
        &mut findings,
    );
    assert_eq!(findings.len(), 2);
    for (message, policy) in [("exact", "exact-policy"), ("contains", "contains-policy")] {
        assert!(findings.iter().any(|finding| matches!(
            finding,
            Finding::Mismatch { key, selector: Some(selector), message: found_message, attribution, .. }
                if key == "values.react"
                    && selector == "react"
                    && found_message == message
                    && attribution == &vec![provenance_named(policy)]
        )));
    }

    let mut excluded_findings = Vec::new();
    let _ = reconcile_list_field(
        "values".to_owned(),
        vec!["react".to_owned(), "blocked".to_owned()],
        &resolved,
        ListFieldKeyStyle::FieldItem,
        &mut excluded_findings,
    );
    assert_eq!(excluded_findings.len(), 2);
    for (message, policy) in [("exact", "exact-policy"), ("excludes", "excludes-policy")] {
        assert!(excluded_findings.iter().any(|finding| matches!(
            finding,
            Finding::Mismatch { key, selector: Some(selector), message: found_message, attribution, .. }
                if key == "values.blocked"
                    && selector == "blocked"
                    && found_message == message
                    && attribution == &vec![provenance_named(policy)]
        )));
    }
}

#[test]
fn report_list_shape_reports_non_array_and_non_string_members() {
    let (doc, findings) = parse_or_report(Some(b"ignore = [\"src\", 1]\n"), "rustfmt.toml");
    assert!(findings.is_empty());

    let mut list = ListRequirements {
        contains: BTreeMap::new(),
        excludes: BTreeMap::new(),
        exact: None,
    };
    let _ = list
        .contains
        .insert("src".to_owned(), "must contain src".to_owned());
    let mut conflicts = Vec::new();
    let resolved = resolve_list("ignore", vec![(provenance(), list)], &mut conflicts);
    let mut shape_findings = Vec::new();

    assert!(report_list_shape(
        &doc,
        "ignore",
        &resolved,
        &mut shape_findings
    ));
    assert!(conflicts.is_empty());
    assert_eq!(shape_findings.len(), 1);
    let (key, expected) = mismatch_key_and_expected(
        shape_findings
            .first()
            .expect("one list-shape mismatch finding"),
    )
    .expect("finding is a mismatch");
    assert_eq!(key, "ignore[1]");
    assert_eq!(expected, "string");

    let item = aqc_toml_engine_core::list_item(&["src".to_owned()]);
    assert!(item.as_array().is_some());
}
