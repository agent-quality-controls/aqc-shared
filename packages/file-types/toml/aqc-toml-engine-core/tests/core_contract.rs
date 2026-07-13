#![allow(
    clippy::expect_used,
    reason = "Tests use expect to fail loudly when fixture invariants are broken."
)]
use std::collections::{BTreeMap, BTreeSet};

use aqc_file_engine_core::{
    ConfigScalar, FileItemRequirement, Finding, ItemAssertionInput, ItemRequirements,
    ListRequirements, Provenance, RequiredItemResolution, ScalarAssertion, compose_item_by,
    resolve_items, resolve_list,
};
use aqc_toml_engine_core::{
    ListFieldKeyStyle, ScalarFieldEdit, TomlArrayItem, TomlArrayTableItem, TomlItemError,
    TomlItemField, parse_or_report, reconcile_array_items, reconcile_array_table_items,
    reconcile_list_field, report_list_shape, scalar_field_edit, table_at, table_list_values,
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
    Provenance {
        policy: "policy".to_owned(),
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
