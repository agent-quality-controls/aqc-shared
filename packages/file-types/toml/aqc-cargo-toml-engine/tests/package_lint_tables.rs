#![allow(
    clippy::expect_used,
    reason = "Tests use expect to expose invalid engine output."
)]

use aqc_toml_engine_core as _;
use globset as _;
use toml_edit as _;

#[allow(
    dead_code,
    unused_imports,
    reason = "Shared integration helpers are compiled per test crate."
)]
mod common;
use common::*;

fn table_item(tool: &str) -> engine_core::KeyedItem<()> {
    engine_core::KeyedItem {
        file_key: tool.to_owned(),
        value: (),
    }
}

fn table_requirements(
    required: Vec<(&str, &str)>,
    forbidden: Vec<(&str, &str)>,
    exact: Option<(Vec<&str>, &str)>,
) -> cargo::CargoTomlRequirements {
    cargo::CargoTomlRequirements {
        package_lint_tables: engine_core::ItemRequirements {
            required: required
                .into_iter()
                .map(|(tool, message)| (table_item(tool), message.to_owned()))
                .collect(),
            forbidden: forbidden
                .into_iter()
                .map(|(tool, message)| (table_item(tool), message.to_owned()))
                .collect(),
            exact: exact.map(|(tools, message)| {
                (
                    tools.into_iter().map(table_item).collect(),
                    message.to_owned(),
                )
            }),
        },
        ..Default::default()
    }
}

fn inline_requirement(tool: &str) -> cargo::CargoTomlRequirements {
    let lint = engine_core::KeyedItem {
        file_key: "unsafe_code".to_owned(),
        value: cargo::LintSetting {
            level: "forbid".to_owned(),
            priority: None,
        },
    };
    cargo::CargoTomlRequirements {
        package_lints: Some(cargo::PackageLintsAssertion::Inline(BTreeMap::from([(
            tool.to_owned(),
            engine_core::ItemRequirements {
                required: vec![(lint, "forbid unsafe".to_owned())],
                forbidden: Vec::new(),
                exact: None,
            },
        )]))),
        ..Default::default()
    }
}

#[test]
fn required_table_reports_missing_and_preserves_unlisted_tables() {
    let findings = cargo_findings_with(
        Some(b"[lints.clippy]\nunwrap_used = \"deny\"\n"),
        vec![(
            prov("rust"),
            table_requirements(vec![("rust", "need rust")], Vec::new(), None),
        )],
    );

    assert_eq!(mismatch_count_for_key(&findings, "[lints.rust]"), 1);
    assert_eq!(mismatch_count_for_key(&findings, "[lints.clippy]"), 0);
}

#[test]
fn required_table_accepts_an_existing_table() {
    let findings = cargo_findings_with(
        Some(b"[lints.rust]\nunsafe_code = \"forbid\"\n"),
        vec![(
            prov("rust"),
            table_requirements(vec![("rust", "need rust")], Vec::new(), None),
        )],
    );
    assert!(findings.is_empty());
}

#[test]
fn forbidden_table_reports_only_the_named_table() {
    let findings = cargo_findings_with(
        Some(b"[lints.rust]\nunsafe_code = \"forbid\"\n[lints.clippy]\nunwrap_used = \"deny\"\n"),
        vec![(
            prov("no-rust"),
            table_requirements(Vec::new(), vec![("rust", "no rust")], None),
        )],
    );

    assert_eq!(mismatch_count_for_key(&findings, "[lints.rust]"), 1);
    assert_eq!(mismatch_count_for_key(&findings, "[lints.clippy]"), 0);
}

#[test]
fn exact_empty_reports_each_local_lint_table_separately() {
    let findings = cargo_findings_with(
        Some(b"[lints.rust]\nunsafe_code = \"forbid\"\n[lints.clippy]\nunwrap_used = \"deny\"\n"),
        vec![(
            prov("inherit"),
            table_requirements(Vec::new(), Vec::new(), Some((Vec::new(), "use workspace"))),
        )],
    );

    assert_eq!(findings.len(), 2);
    assert_eq!(mismatch_count_for_key(&findings, "[lints.rust]"), 1);
    assert_eq!(mismatch_count_for_key(&findings, "[lints.clippy]"), 1);
}

#[test]
fn exact_nonempty_reports_missing_and_extra_tables() {
    let findings = cargo_findings_with(
        Some(b"[lints.clippy]\nunwrap_used = \"deny\"\n"),
        vec![(
            prov("exact"),
            table_requirements(Vec::new(), Vec::new(), Some((vec!["rust"], "rust only"))),
        )],
    );

    assert_eq!(mismatch_count_for_key(&findings, "[lints.rust]"), 1);
    assert_eq!(mismatch_count_for_key(&findings, "[lints.clippy]"), 1);
}

#[test]
fn scalar_under_lints_is_not_a_package_lint_table_identity() {
    let findings = cargo_findings_with(
        Some(b"[lints]\nworkspace = true\n"),
        vec![(
            prov("exact"),
            table_requirements(
                Vec::new(),
                Vec::new(),
                Some((Vec::new(), "no local tables")),
            ),
        )],
    );
    assert!(findings.is_empty());
}

#[test]
fn inline_lint_requirement_implies_required_table_presence() {
    let (resolved, conflicts) =
        cargo::CargoTomlRequirements::merge(vec![(prov("inline"), inline_requirement("rust"))]);

    assert!(conflicts.is_empty());
    assert!(resolved.package_lint_tables.required.contains_key("rust"));
}

#[test]
fn inline_lint_requirement_accepts_existing_table_and_content() {
    let findings = cargo_findings_with(
        Some(b"[lints.rust]\nunsafe_code = \"forbid\"\n"),
        vec![(prov("inline"), inline_requirement("rust"))],
    );
    assert!(findings.is_empty());
}

#[test]
fn exact_empty_conflicts_with_inline_implied_table_presence() {
    let (_, conflicts) = cargo::CargoTomlRequirements::merge(vec![
        (
            prov("inherit"),
            table_requirements(
                Vec::new(),
                Vec::new(),
                Some((Vec::new(), "no local tables")),
            ),
        ),
        (prov("inline"), inline_requirement("rust")),
    ]);

    assert!(conflicts.iter().any(|conflict| {
        conflict.key == "[lints].rust"
            && conflict.reason == "exact-items-reject-unlisted-required-item"
            && conflict
                .contributors
                .iter()
                .any(|(source, message)| source.policy == "inherit" && message == "no local tables")
            && conflict
                .contributors
                .iter()
                .any(|(source, value)| source.policy == "inline" && value == "required")
    }));
}

#[test]
fn forbidden_table_conflicts_with_inline_implied_table_presence() {
    let (_, conflicts) = cargo::CargoTomlRequirements::merge(vec![
        (
            prov("forbid"),
            table_requirements(Vec::new(), vec![("rust", "no rust")], None),
        ),
        (prov("inline"), inline_requirement("rust")),
    ]);

    assert!(
        conflicts
            .iter()
            .any(|conflict| conflict.key == "[lints].rust"
                && conflict.reason == "item-required-and-forbidden")
    );
}

#[test]
fn matching_exact_and_inline_requirements_compose() {
    let (_, conflicts) = cargo::CargoTomlRequirements::merge(vec![
        (
            prov("exact"),
            table_requirements(Vec::new(), Vec::new(), Some((vec!["rust"], "rust only"))),
        ),
        (prov("inline"), inline_requirement("rust")),
    ]);
    assert!(conflicts.is_empty());
}

#[test]
fn agreeing_exact_table_sets_merge_independent_of_order() {
    let (resolved, conflicts) = cargo::CargoTomlRequirements::merge(vec![
        (
            prov("left"),
            table_requirements(
                Vec::new(),
                Vec::new(),
                Some((vec!["rust", "clippy"], "left exact")),
            ),
        ),
        (
            prov("right"),
            table_requirements(
                Vec::new(),
                Vec::new(),
                Some((vec!["clippy", "rust"], "right exact")),
            ),
        ),
    ]);

    assert!(conflicts.is_empty());
    assert_eq!(
        resolved
            .package_lint_tables
            .exact
            .expect("exact tables should resolve")
            .identities,
        BTreeSet::from(["clippy".to_owned(), "rust".to_owned()])
    );
}

#[test]
fn differing_exact_table_sets_conflict() {
    let (_, conflicts) = cargo::CargoTomlRequirements::merge(vec![
        (
            prov("left"),
            table_requirements(Vec::new(), Vec::new(), Some((vec!["rust"], "rust only"))),
        ),
        (
            prov("right"),
            table_requirements(
                Vec::new(),
                Vec::new(),
                Some((vec!["clippy"], "clippy only")),
            ),
        ),
    ]);

    assert!(conflicts.iter().any(|conflict| {
        conflict.key == "[lints]" && conflict.reason == "exact-item-identities-disagree"
    }));
}

#[test]
fn inheritance_does_not_itself_imply_a_local_table_requirement() {
    let req = cargo::CargoTomlRequirements {
        package_lints: Some(cargo::PackageLintsAssertion::Inherit(
            true,
            "inherit".to_owned(),
        )),
        ..Default::default()
    };
    let (resolved, conflicts) = cargo::CargoTomlRequirements::merge(vec![(prov("inherit"), req)]);

    assert!(conflicts.is_empty());
    assert!(resolved.package_lint_tables.required.is_empty());
    assert!(resolved.package_lint_tables.exact.is_none());
}
