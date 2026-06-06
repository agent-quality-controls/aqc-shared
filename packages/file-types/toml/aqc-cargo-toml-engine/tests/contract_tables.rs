//! The from-empty contract catalogue, part 2: dependency-shaped targets,
//! features, profiles, target tables, and patch. Split from `contract.rs` to
//! keep file size in bounds; same `contract::` module path.

use toml_edit as _;

mod contract {
    use std::collections::{BTreeMap, BTreeSet};

    use aqc_cargo_toml_engine::{
        CargoTomlEngine, CargoTomlRequirement, DependencyKind, DependencyScope,
        DependencySetAssertion, DependencySpec, FeatureSetAssertion, ProfileAssertion,
        ProfileFieldAssertion, TargetFieldAssertion, TargetTableAssertion,
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

    /// Short helper for owned fixture messages.
    fn msg() -> String {
        "fixture message".to_owned()
    }

    /// A requirement with one `[lib].<field>` assertion; runs its law.
    fn lib_field_law(field: &str, a: TargetFieldAssertion, what: &str) {
        let class = a.on_empty();
        let mut req = CargoTomlRequirement::default();
        let _ = req.lib_fields.insert(field.to_owned(), ma(a));
        law(&req, class, what);
    }

    // ---------------- Dependencies (source rule + variants + scopes) ----------------

    /// One dependency entry map with the given spec.
    fn dep_entries(spec: DependencySpec) -> aqc_cargo_toml_engine::requirement::DependencyEntries {
        BTreeMap::from([("serde".to_owned(), (spec, msg()))])
    }

    /// A requirement with one dependency assertion in the given scope.
    fn dep_req(scope: DependencyScope, a: DependencySetAssertion) -> CargoTomlRequirement {
        let mut req = CargoTomlRequirement::default();
        let _ = req.dependencies.insert(scope, ma(a));
        req
    }

    /// The plain `[dependencies]` scope.
    const fn normal_scope() -> DependencyScope {
        DependencyScope {
            kind: DependencyKind::Normal,
            target: None,
        }
    }

    #[test]
    fn dependency_contains_with_source_writes() {
        let spec = DependencySpec {
            version: Some("1.0".to_owned()),
            ..DependencySpec::default()
        };
        let a = DependencySetAssertion::Contains(dep_entries(spec));
        assert_eq!(
            a.on_empty(),
            FromEmpty::Writes,
            "a sourced spec is writable"
        );
        let req = dep_req(normal_scope(), a.clone());
        law(&req, a.on_empty(), "dependency Contains with source");
        let out = reconcile(None, &req);
        let text = String::from_utf8(out.expected_bytes).expect("engine output is utf-8");
        assert!(
            text.contains("serde = \"1.0\""),
            "version-only specs write the string shorthand: {text}"
        );
    }

    #[test]
    fn dependency_contains_without_source_checks_only() {
        let spec = DependencySpec {
            features: vec!["derive".to_owned()],
            ..DependencySpec::default()
        };
        let a = DependencySetAssertion::Contains(dep_entries(spec));
        assert_eq!(
            a.on_empty(),
            FromEmpty::ChecksOnly,
            "a spec with no source cannot create the entry"
        );
        let req = dep_req(normal_scope(), a.clone());
        law(&req, a.on_empty(), "dependency Contains without source");
    }

    #[test]
    fn dependency_workspace_inherit_writes_inline() {
        let spec = DependencySpec {
            workspace: Some(true),
            ..DependencySpec::default()
        };
        let a = DependencySetAssertion::Contains(dep_entries(spec));
        let req = dep_req(normal_scope(), a.clone());
        law(&req, a.on_empty(), "dependency workspace inherit");
        let out = reconcile(None, &req);
        let text = String::from_utf8(out.expected_bytes).expect("engine output is utf-8");
        assert!(
            text.contains("serde = { workspace = true }"),
            "the workspace inheritance form is written: {text}"
        );
    }

    #[test]
    fn dependency_excludes_and_is_exactly() {
        let excl =
            DependencySetAssertion::Excludes(BTreeMap::from([("openssl".to_owned(), msg())]));
        law(
            &dep_req(normal_scope(), excl.clone()),
            excl.on_empty(),
            "dependency Excludes",
        );
        let spec = DependencySpec {
            version: Some("1.0".to_owned()),
            ..DependencySpec::default()
        };
        let exact = DependencySetAssertion::IsExactly(dep_entries(spec));
        law(
            &dep_req(normal_scope(), exact.clone()),
            exact.on_empty(),
            "dependency IsExactly",
        );
    }

    #[test]
    fn dependency_target_cfg_scope_writes() {
        let spec = DependencySpec {
            version: Some("0.3".to_owned()),
            ..DependencySpec::default()
        };
        let scope = DependencyScope {
            kind: DependencyKind::Dev,
            target: Some("cfg(windows)".to_owned()),
        };
        let a = DependencySetAssertion::Contains(BTreeMap::from([(
            "winapi".to_owned(),
            (spec, msg()),
        )]));
        let req = dep_req(scope, a.clone());
        law(&req, a.on_empty(), "dependency in a cfg target scope");
        let out = reconcile(None, &req);
        let text = String::from_utf8(out.expected_bytes).expect("engine output is utf-8");
        assert!(
            text.contains("winapi"),
            "the cfg-scoped table is written: {text}"
        );
    }

    #[test]
    fn workspace_dependencies_optional_is_schema_error() {
        let spec = DependencySpec {
            version: Some("1.0".to_owned()),
            optional: Some(true),
            ..DependencySpec::default()
        };
        let req = CargoTomlRequirement {
            workspace_dependencies: Some(ma(DependencySetAssertion::Contains(dep_entries(spec)))),
            ..CargoTomlRequirement::default()
        };
        let out = reconcile(None, &req);
        assert!(
            out.findings
                .iter()
                .any(|f| matches!(f, aqc_file_engine_core::Finding::SchemaError { .. })),
            "optional in [workspace.dependencies] is a SchemaError: {:?}",
            out.findings
        );
    }

    // ---------------- FeatureSetAssertion (3 variants) ----------------

    #[test]
    fn feature_variants() {
        let contains = FeatureSetAssertion::Contains(BTreeMap::from([(
            "full".to_owned(),
            (BTreeSet::from(["std".to_owned()]), msg()),
        )]));
        let excludes =
            FeatureSetAssertion::Excludes(BTreeMap::from([("nightly".to_owned(), msg())]));
        let exact = FeatureSetAssertion::IsExactly(BTreeMap::from([(
            "default".to_owned(),
            (BTreeSet::new(), msg()),
        )]));
        for (what, a) in [
            ("features Contains", contains),
            ("features Excludes", excludes),
            ("features IsExactly", exact),
        ] {
            let class = a.on_empty();
            let req = CargoTomlRequirement {
                features: Some(ma(a)),
                ..CargoTomlRequirement::default()
            };
            law(&req, class, what);
        }
    }

    // ---------------- Profiles (field variants + overrides) ----------------

    #[test]
    fn profile_field_variants() {
        for (what, a) in [
            (
                "profile Equals",
                ProfileFieldAssertion::Equals(ConfigScalar::Str("thin".to_owned()), msg()),
            ),
            (
                "profile OneOf",
                ProfileFieldAssertion::OneOf(
                    vec![
                        ConfigScalar::Str("thin".to_owned()),
                        ConfigScalar::Bool(true),
                    ],
                    msg(),
                ),
            ),
            ("profile Present", ProfileFieldAssertion::Present(msg())),
            ("profile Absent", ProfileFieldAssertion::Absent(msg())),
        ] {
            let assertion = ProfileAssertion {
                fields: BTreeMap::from([("lto".to_owned(), a)]),
                package_overrides: BTreeMap::new(),
                build_override: BTreeMap::new(),
            };
            let class = assertion.on_empty();
            let mut req = CargoTomlRequirement::default();
            let _ = req.profiles.insert("release".to_owned(), ma(assertion));
            law(&req, class, what);
        }
    }

    #[test]
    fn profile_overrides_write() {
        let assertion = ProfileAssertion {
            fields: BTreeMap::new(),
            package_overrides: BTreeMap::from([(
                "*".to_owned(),
                BTreeMap::from([(
                    "opt-level".to_owned(),
                    ProfileFieldAssertion::Equals(ConfigScalar::Int(3), msg()),
                )]),
            )]),
            build_override: BTreeMap::from([(
                "debug".to_owned(),
                ProfileFieldAssertion::Equals(ConfigScalar::Bool(false), msg()),
            )]),
        };
        let class = assertion.on_empty();
        let mut req = CargoTomlRequirement::default();
        let _ = req.profiles.insert("dev".to_owned(), ma(assertion));
        law(&req, class, "profile package_overrides + build_override");
        let out = reconcile(None, &req);
        let text = String::from_utf8(out.expected_bytes).expect("engine output is utf-8");
        assert!(
            text.contains("opt-level = 3") && text.contains("debug = false"),
            "override tables are written: {text}"
        );
    }

    // ---------------- Target tables ----------------

    #[test]
    fn lib_field_variants() {
        lib_field_law(
            "doctest",
            TargetFieldAssertion::Equals(ConfigScalar::Bool(true), msg()),
            "lib Equals",
        );
        lib_field_law(
            "crate-type",
            TargetFieldAssertion::OneOf(BTreeSet::from(["rlib".to_owned()]), msg()),
            "lib OneOf",
        );
        lib_field_law(
            "crate-type",
            TargetFieldAssertion::ListContains(vec!["rlib".to_owned()], msg()),
            "lib ListContains",
        );
        lib_field_law(
            "crate-type",
            TargetFieldAssertion::ListIsExactly(vec!["rlib".to_owned()], msg()),
            "lib ListIsExactly",
        );
        lib_field_law("path", TargetFieldAssertion::Present(msg()), "lib Present");
        lib_field_law("plugin", TargetFieldAssertion::Absent(msg()), "lib Absent");
    }

    #[test]
    fn bin_target_created_by_name() {
        let a = TargetTableAssertion::Present(msg());
        let class = a.on_empty();
        let mut req = CargoTomlRequirement::default();
        let _ = req.bin_targets.insert("g3rs".to_owned(), ma(a));
        law(&req, class, "bin Present");
        let out = reconcile(None, &req);
        let text = String::from_utf8(out.expected_bytes).expect("engine output is utf-8");
        assert!(
            text.contains("[[bin]]") && text.contains("name = \"g3rs\""),
            "the entry is created with its name: {text}"
        );
    }

    #[test]
    fn bin_target_absent_and_fields() {
        let absent = TargetTableAssertion::Absent(msg());
        let mut req = CargoTomlRequirement::default();
        let _ = req.bin_targets.insert("old".to_owned(), ma(absent.clone()));
        law(&req, absent.on_empty(), "bin Absent");

        let fields = TargetTableAssertion::Fields(BTreeMap::from([(
            "test".to_owned(),
            TargetFieldAssertion::Equals(ConfigScalar::Bool(false), msg()),
        )]));
        let class = fields.on_empty();
        assert_eq!(class, FromEmpty::Writes, "all-writable Fields is writable");
        let mut fields_req = CargoTomlRequirement::default();
        let _ = fields_req.bin_targets.insert("g3rs".to_owned(), ma(fields));
        law(&fields_req, class, "bin Fields (writable)");
    }

    #[test]
    fn bin_target_fields_with_check_only_field_checks_only() {
        let fields = TargetTableAssertion::Fields(BTreeMap::from([(
            "path".to_owned(),
            TargetFieldAssertion::Present(msg()),
        )]));
        assert_eq!(
            fields.on_empty(),
            FromEmpty::ChecksOnly,
            "a check-only field makes the entry check-only"
        );
        let mut req = CargoTomlRequirement::default();
        let _ = req
            .bin_targets
            .insert("g3rs".to_owned(), ma(fields.clone()));
        law(&req, fields.on_empty(), "bin Fields (check-only)");
    }

    // ---------------- Patch ----------------

    #[test]
    fn patch_registry_writes() {
        let spec = DependencySpec {
            git: Some("https://github.com/serde-rs/serde".to_owned()),
            ..DependencySpec::default()
        };
        let a = DependencySetAssertion::Contains(dep_entries(spec));
        let class = a.on_empty();
        let mut req = CargoTomlRequirement::default();
        let _ = req.patch.insert("crates-io".to_owned(), ma(a));
        law(&req, class, "patch Contains");
        let out = reconcile(None, &req);
        let text = String::from_utf8(out.expected_bytes).expect("engine output is utf-8");
        assert!(
            text.contains("[patch.crates-io]") || text.contains("[patch.\"crates-io\"]"),
            "the patch table is written: {text}"
        );
    }
}
