//! The from-empty contract catalogue: every variant of every assertion enum
//! runs through the core harness, asserting its DECLARED class (`on_empty`)
//! matches its ACTUAL reconcile behavior. A wrong declaration or a wrong
//! apply fails here.

use toml_edit as _;

mod contract {
    use std::collections::{BTreeMap, BTreeSet};

    use aqc_cargo_toml_engine::{
        CargoTomlEngine, CargoTomlRequirement, LintLevelsAssertion, LintsInheritAssertion,
        ManifestSection, PackageFieldAssertion, SectionPresenceAssertion, WorkspaceFieldAssertion,
    };
    use aqc_file_engine_core::{
        ConfigScalar, FileEngine, FromEmpty, FromEmptyClass, MergedAssertion, Provenance,
        check_from_empty,
    };

    /// The engine's typed reconcile, as the harness expects it.
    fn reconcile(
        current: Option<&[u8]>,
        req: &CargoTomlRequirement,
    ) -> aqc_file_engine_core::EngineOutput {
        <CargoTomlEngine as FileEngine<CargoTomlRequirement>>::reconcile(current, req)
    }

    /// Wrap one assertion as a single-policy `MergedAssertion`.
    fn ma<A>(a: A) -> MergedAssertion<A> {
        MergedAssertion {
            contributions: vec![(
                Provenance {
                    policy: "fixture".to_owned(),
                },
                a,
            )],
        }
    }

    /// Run the harness with the assertion's own declared class.
    fn law(req: &CargoTomlRequirement, class: FromEmpty, what: &str) {
        let outcome = check_from_empty(reconcile, req, class);
        assert!(outcome.is_ok(), "{what}: {outcome:?}");
    }

    /// A requirement with one `[package].<field>` assertion; runs its law.
    fn package_field_law(field: &str, a: PackageFieldAssertion, what: &str) {
        let class = a.on_empty();
        let mut req = CargoTomlRequirement::default();
        let _ = req.package_fields.insert(field.to_owned(), ma(a));
        law(&req, class, what);
    }

    /// A requirement with one `[workspace].<key>` assertion; runs its law.
    fn workspace_field_law(key: &str, a: WorkspaceFieldAssertion, what: &str) {
        let class = a.on_empty();
        let mut req = CargoTomlRequirement::default();
        let _ = req.workspace_fields.insert(key.to_owned(), ma(a));
        law(&req, class, what);
    }

    const MSG: &str = "fixture message";

    /// Short helper for owned fixture messages.
    fn msg() -> String {
        MSG.to_owned()
    }

    // ---------------- PackageFieldAssertion (9 variants) ----------------

    #[test]
    fn package_equals_writes() {
        package_field_law(
            "edition",
            PackageFieldAssertion::Equals(ConfigScalar::Str("2021".to_owned()), msg()),
            "package Equals(Str)",
        );
        package_field_law(
            "autobins",
            PackageFieldAssertion::Equals(ConfigScalar::Bool(false), msg()),
            "package Equals(Bool)",
        );
    }

    #[test]
    fn package_at_least_version_writes() {
        package_field_law(
            "rust-version",
            PackageFieldAssertion::AtLeastVersion("1.85".to_owned(), msg()),
            "package AtLeastVersion",
        );
    }

    #[test]
    fn package_at_least_version_raises_below_floor() {
        let mut req = CargoTomlRequirement::default();
        let _ = req.package_fields.insert(
            "rust-version".to_owned(),
            ma(PackageFieldAssertion::AtLeastVersion(
                "1.85".to_owned(),
                msg(),
            )),
        );
        let out = reconcile(Some(b"[package]\nrust-version = \"1.70\"\n"), &req);
        let text = String::from_utf8(out.expected_bytes).expect("engine output is utf-8");
        assert!(
            text.contains("rust-version = \"1.85\""),
            "a below-floor version is raised to the floor: {text}"
        );
    }

    #[test]
    fn package_one_of_checks_only() {
        package_field_law(
            "license",
            PackageFieldAssertion::OneOf(
                BTreeSet::from(["MIT".to_owned(), "Apache-2.0".to_owned()]),
                msg(),
            ),
            "package OneOf",
        );
    }

    #[test]
    fn package_list_contains_writes() {
        package_field_law(
            "keywords",
            PackageFieldAssertion::ListContains(vec!["cli".to_owned()], msg()),
            "package ListContains",
        );
    }

    #[test]
    fn package_list_excludes_writes() {
        package_field_law(
            "keywords",
            PackageFieldAssertion::ListExcludes(BTreeSet::from(["bad".to_owned()]), msg()),
            "package ListExcludes",
        );
    }

    #[test]
    fn package_list_is_exactly_writes() {
        package_field_law(
            "categories",
            PackageFieldAssertion::ListIsExactly(vec!["command-line-utilities".to_owned()], msg()),
            "package ListIsExactly",
        );
    }

    #[test]
    fn package_inherits_workspace_writes_inline_form() {
        package_field_law(
            "version",
            PackageFieldAssertion::InheritsWorkspace(msg()),
            "package InheritsWorkspace",
        );
        let mut req = CargoTomlRequirement::default();
        let _ = req.package_fields.insert(
            "version".to_owned(),
            ma(PackageFieldAssertion::InheritsWorkspace(msg())),
        );
        let out = reconcile(None, &req);
        let text = String::from_utf8(out.expected_bytes).expect("engine output is utf-8");
        assert!(
            text.contains("version = { workspace = true }"),
            "the inheritance write form is the inline table: {text}"
        );
    }

    #[test]
    fn package_present_checks_only() {
        package_field_law(
            "description",
            PackageFieldAssertion::Present(msg()),
            "package Present",
        );
    }

    #[test]
    fn package_absent_writes() {
        package_field_law(
            "authors",
            PackageFieldAssertion::Absent(msg()),
            "package Absent",
        );
    }

    // ---------------- WorkspaceFieldAssertion (7 variants) ----------------

    #[test]
    fn workspace_field_variants() {
        workspace_field_law(
            "resolver",
            WorkspaceFieldAssertion::Equals(ConfigScalar::Str("3".to_owned()), msg()),
            "workspace Equals",
        );
        workspace_field_law(
            "resolver",
            WorkspaceFieldAssertion::OneOf(BTreeSet::from(["2".to_owned(), "3".to_owned()]), msg()),
            "workspace OneOf",
        );
        workspace_field_law(
            "members",
            WorkspaceFieldAssertion::ListContains(vec!["packages/*".to_owned()], msg()),
            "workspace ListContains",
        );
        workspace_field_law(
            "exclude",
            WorkspaceFieldAssertion::ListExcludes(BTreeSet::from(["target".to_owned()]), msg()),
            "workspace ListExcludes",
        );
        workspace_field_law(
            "default-members",
            WorkspaceFieldAssertion::ListIsExactly(vec!["packages/app".to_owned()], msg()),
            "workspace ListIsExactly",
        );
        workspace_field_law(
            "resolver",
            WorkspaceFieldAssertion::Present(msg()),
            "workspace Present",
        );
        workspace_field_law(
            "exclude",
            WorkspaceFieldAssertion::Absent(msg()),
            "workspace Absent",
        );
    }

    // ---------------- SectionPresence (section-dependent class) ----------------

    #[test]
    fn section_presence_workspace_present_writes_empty_table() {
        let a = SectionPresenceAssertion::Present(msg());
        let class = a.on_empty_in(ManifestSection::Workspace);
        assert_eq!(class, FromEmpty::Writes, "Present([workspace]) is writable");
        let mut req = CargoTomlRequirement::default();
        let _ = req
            .section_presence
            .insert(ManifestSection::Workspace, ma(a));
        law(&req, class, "section Present(Workspace)");
        let out = reconcile(None, &req);
        let text = String::from_utf8(out.expected_bytes).expect("engine output is utf-8");
        assert!(
            text.contains("[workspace]"),
            "an empty [workspace] table is written: {text}"
        );
    }

    #[test]
    fn section_presence_package_present_checks_only() {
        let a = SectionPresenceAssertion::Present(msg());
        let class = a.on_empty_in(ManifestSection::Package);
        assert_eq!(
            class,
            FromEmpty::ChecksOnly,
            "Present([package]) cannot invent a name"
        );
        let mut req = CargoTomlRequirement::default();
        let _ = req.section_presence.insert(ManifestSection::Package, ma(a));
        law(&req, class, "section Present(Package)");
    }

    #[test]
    fn section_presence_absent_writes() {
        let a = SectionPresenceAssertion::Absent(msg());
        let class = a.on_empty_in(ManifestSection::Replace);
        let mut req = CargoTomlRequirement::default();
        let _ = req.section_presence.insert(ManifestSection::Replace, ma(a));
        law(&req, class, "section Absent(Replace)");
    }

    // ---------------- LintLevelsAssertion (3 variants) ----------------

    /// The lint map for one entry.
    #[expect(
        clippy::type_complexity,
        reason = "BTreeMap<String, (level, priority, message)> mirrors `LintLevelsAssertion::Contains`'s value shape."
    )]
    fn lint_entry() -> BTreeMap<String, (String, Option<i64>, String)> {
        BTreeMap::from([("unwrap_used".to_owned(), ("deny".to_owned(), None, msg()))])
    }

    #[test]
    fn lint_levels_variants() {
        for (what, assertion) in [
            (
                "lints Contains",
                LintLevelsAssertion::Contains(lint_entry()),
            ),
            (
                "lints Excludes",
                LintLevelsAssertion::Excludes(BTreeMap::from([("bad_lint".to_owned(), msg())])),
            ),
            (
                "lints IsExactly",
                LintLevelsAssertion::IsExactly(lint_entry()),
            ),
        ] {
            let class = assertion.on_empty();
            let mut req = CargoTomlRequirement::default();
            let _ = req
                .workspace_lints
                .insert("clippy".to_owned(), ma(assertion));
            law(&req, class, what);
        }
    }

    // ---------------- LintsInheritAssertion (3 variants + exclusivity) ----------------

    #[test]
    fn lints_inherit_variants() {
        for (what, assertion) in [
            (
                "lints_inherit Equals",
                LintsInheritAssertion::Equals(true, msg()),
            ),
            (
                "lints_inherit Present",
                LintsInheritAssertion::Present(msg()),
            ),
            ("lints_inherit Absent", LintsInheritAssertion::Absent(msg())),
        ] {
            let class = assertion.on_empty();
            let req = CargoTomlRequirement {
                lints_inherit: Some(ma(assertion)),
                ..CargoTomlRequirement::default()
            };
            law(&req, class, what);
        }
    }

    #[test]
    fn lints_inherit_with_inline_lints_is_schema_error() {
        let mut req = CargoTomlRequirement {
            lints_inherit: Some(ma(LintsInheritAssertion::Equals(true, msg()))),
            ..CargoTomlRequirement::default()
        };
        let _ = req.lints.insert(
            "clippy".to_owned(),
            ma(LintLevelsAssertion::Contains(lint_entry())),
        );
        let out = reconcile(None, &req);
        assert!(
            out.findings
                .iter()
                .any(|f| matches!(f, aqc_file_engine_core::Finding::SchemaError { .. })),
            "the incompatible combination is a SchemaError: {:?}",
            out.findings
        );
        let text = String::from_utf8(out.expected_bytes).expect("engine output is utf-8");
        assert!(
            !text.contains("[lints."),
            "no inline lint table is written for the rejected combination: {text}"
        );
        assert!(
            !text.contains("workspace = true"),
            "the inherit key is not written for the rejected combination: {text}"
        );
    }
}
