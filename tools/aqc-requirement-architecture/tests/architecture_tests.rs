use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use aqc_requirement_architecture::{RequirementKind, ViolationCode, check_repository_roots};

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

fn core_manifest() -> PathBuf {
    fixture("accepted/core").join("Cargo.toml")
}

fn report(name: &str) -> aqc_requirement_architecture::ArchitectureReport {
    let roots = if name == "rejected" {
        vec![fixture(name), fixture("accepted/external-membership")]
    } else {
        vec![fixture(name)]
    };
    check_repository_roots(&core_manifest(), &roots)
        .expect("The architecture fixture must produce an architecture report.")
}

fn violations_for_function(name: &str) -> Vec<String> {
    report("rejected")
        .violations
        .into_iter()
        .filter(|violation| violation.message.contains(&format!("function {name}")))
        .map(|violation| violation.message)
        .collect()
}

#[test]
fn rejected_fixture_covers_membership_construction_and_disguised_roots() {
    let report = check_repository_roots(
        &core_manifest(),
        &[fixture("rejected"), fixture("accepted/external-membership")],
    )
    .expect("The rejected fixture must produce an architecture report.");
    let codes = report
        .violations
        .iter()
        .map(|violation| violation.code)
        .collect::<BTreeSet<_>>();
    assert_rejected_codes(&codes);
    let messages = report
        .violations
        .iter()
        .map(|violation| violation.message.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    assert_expected_messages(&messages);
    assert_expected_functions(&messages);
    assert!(
        report.roots.iter().any(|root| root.name == "HiddenAdapter"),
        "Renamed AdapterRequirement imports must not hide requirement roots."
    );
}

fn assert_expected_messages(messages: &str) {
    for expected in [
        "constructs ItemRequirements",
        "macro",
        "mutating or unrecognized method",
        "borrows a membership collection mutably",
        "destructures membership internals",
        "PrivateClosureField field exact_settings",
        "PrivateNestedClosure field exact_settings",
        "reimplements core vocabulary type ItemRequirements",
        "renames core vocabulary type ItemRequirements",
        "module-level macro hidden_requirement_root",
        "ambiguous local type Child",
        "WrappedClosureEngine field closure",
        "NoncanonicalMembershipField field membership",
        "TupleRequirementRoot uses unnamed",
        "AliasedRequirementRoot is a type alias",
    ] {
        assert!(
            messages.contains(expected),
            "Missing adversarial case: {expected}."
        );
    }
}

fn assert_expected_functions(messages: &str) {
    for function in [
        "direct_exact",
        "nested_keys",
        "inferred_values",
        "optional_filter",
        "represented_discovery",
        "assigned_exact",
        "assigned_allowed",
        "renamed_local_membership_mutation",
        "hidden_default_construction",
        "cross_crate_membership_helper",
        "cross_crate_wrapper_field",
        "cross_crate_engine_membership_helper",
        "shadowed_membership_transfer",
        "tuple_shadowed_membership_transfer",
        "rewrite_membership_parameter",
        "membership_helper_parameter",
        "discard_policy_membership",
        "same_name_destructuring_smuggle",
        "local_whole_engine_helper",
        "local_bound_whole_engine_helper",
        "conditional_whole_engine_helper",
        "tuple_whole_engine_helper",
        "reassigned_whole_engine_helper",
        "tuple_struct_whole_engine_helper",
        "cross_crate_whole_engine_helper",
        "destructured_policy_membership_discard",
        "discard_through_self",
        "qualified_same_name_destructuring",
        "replace_whole_membership",
        "borrow_whole_membership",
        "helper_returned_default",
        "replace_dereferenced_membership",
        "borrow_dereferenced_membership",
        "inferred_required",
        "inferred_by_extend",
        "inferred_by_mutable_reference",
        "inferred_by_destructuring",
        "default_membership_replacement",
        "local_macro_alias",
        "closure_membership_parameter",
        "inferred_closure_membership",
        "qualified_local_adapter_discard",
        "tuple_closure_alias_membership",
        "imported_adapter_alias_discard",
        "referenced_closure_alias_membership",
    ] {
        assert!(
            messages.contains(&format!("function {function}")),
            "The adversarial case {function} must produce its own violation."
        );
    }
}

fn assert_rejected_codes(codes: &BTreeSet<ViolationCode>) {
    for (code, purpose) in [
        (ViolationCode::SemanticClosureField, "closure fields"),
        (
            ViolationCode::AdapterMembershipConstruction,
            "adapter membership construction",
        ),
        (
            ViolationCode::NonCanonicalRequirementRoot,
            "aliased and unnamed requirement roots",
        ),
        (
            ViolationCode::ReimplementedCoreVocabulary,
            "local copies of core vocabulary",
        ),
        (
            ViolationCode::UninspectableRequirementMacro,
            "module-level requirement macros",
        ),
        (
            ViolationCode::UninspectableRequirementImport,
            "glob and block-local requirement imports",
        ),
    ] {
        assert!(codes.contains(&code), "The checker must reject {purpose}.");
    }
}

#[test]
fn policy_construction_and_adapter_map_are_accepted() {
    let report = check_repository_roots(&core_manifest(), &[fixture("accepted")])
        .expect("The accepted fixtures must produce an architecture report.");
    assert!(
        report.violations.is_empty(),
        "Policy construction, imported_membership_alias, adapter map, neutral engine defaults, unrelated field mutations, and unrelated macros must pass: {:?}",
        report.violations
    );
}

#[test]
fn policy_allowed_construction_is_accepted() {
    let report = check_repository_roots(&core_manifest(), &[fixture("accepted")])
        .expect("The accepted fixtures must produce an architecture report.");
    assert!(report.violations.is_empty(), "{:#?}", report.violations);
}

#[test]
fn semantic_type_aliases_are_resolved_in_their_declaring_module() {
    let report = report("accepted");
    let root = report
        .roots
        .iter()
        .find(|root| root.name == "ScopedAliasEngine")
        .expect("The engine root using a local semantic alias must be inventoried.");
    assert_eq!(root.membership_fields.len(), 1);
    assert_eq!(root.membership_fields[0].name, "setting_keys");
}

#[test]
fn qualified_local_adapter_root_is_tracked() {
    let violations = violations_for_function("qualified_local_adapter_discard");
    assert!(
        violations
            .iter()
            .any(|message| message.contains("constructs or obtains membership")),
        "A qualified local adapter root must not hide discarded policy membership."
    );
}

#[test]
fn local_helper_produced_engine_membership_is_rejected() {
    let violations = violations_for_function("local_engine_membership_helper");
    assert_eq!(
        violations.len(),
        1,
        "The local helper output must produce one field-transfer violation."
    );
    assert!(
        violations[0].contains("constructs or obtains membership"),
        "The local helper violation must identify hidden membership production."
    );
}

#[test]
fn cross_crate_helper_produced_engine_membership_is_rejected() {
    let violations = violations_for_function("cross_crate_engine_membership_helper");
    assert_eq!(
        violations.len(),
        1,
        "The cross-crate helper output must produce one field-transfer violation."
    );
    assert!(
        violations[0].contains("constructs or obtains membership"),
        "The cross-crate helper violation must identify hidden membership production."
    );
}

#[test]
fn type_annotated_membership_local_is_tracked_and_rejected() {
    let violations = violations_for_function("type_annotated_membership_local_is_tracked");
    assert!(
        violations
            .iter()
            .any(|message| message.contains("mutates a membership collection")),
        "A typed membership local must remain tracked through mutation."
    );
    assert!(
        violations
            .iter()
            .any(|message| message.contains("constructs or obtains membership")),
        "A helper-produced typed local must not become a valid transfer."
    );
}

#[test]
fn whole_engine_requirement_helpers_are_rejected() {
    for function in [
        "local_whole_engine_helper",
        "local_bound_whole_engine_helper",
        "conditional_whole_engine_helper",
        "tuple_whole_engine_helper",
        "reassigned_whole_engine_helper",
        "tuple_struct_whole_engine_helper",
        "cross_crate_whole_engine_helper",
    ] {
        let violations = violations_for_function(function);
        assert_eq!(
            violations.len(),
            1,
            "A whole engine requirement returned by a helper must be rejected."
        );
        assert!(violations[0].contains("engine requirement through a helper"));
    }
}

#[test]
fn adapter_self_and_qualified_impostor_cannot_discard_membership() {
    for function in ["discard_through_self", "qualified_same_name_destructuring"] {
        let violations = violations_for_function(function);
        assert!(
            !violations.is_empty(),
            "The adapter membership in {function} must not be discarded."
        );
    }
}

#[test]
fn destructured_policy_membership_discard_is_rejected() {
    let violations = violations_for_function("destructured_policy_membership_discard");
    assert!(
        violations
            .iter()
            .any(|message| message.contains("constructs or obtains membership")),
        "Destructuring must not hide that policy membership was discarded."
    );
}

#[test]
fn unrelated_same_name_fields_are_accepted() {
    let accepted = report("accepted");
    assert!(
        accepted.violations.is_empty(),
        "Ordinary construction, mutation, and destructuring of same-name fields must pass."
    );
}

#[test]
fn unrelated_keys_field_is_accepted() {
    let accepted = report("accepted");
    assert!(
        accepted.violations.is_empty(),
        "An unrelated Vec-backed cache_keys field must remain outside membership rules."
    );
    let root = accepted
        .roots
        .iter()
        .find(|root| root.name == "UnrelatedKeysEngine")
        .expect("The unrelated keys engine must be inventoried.");
    assert!(
        root.membership_fields.is_empty(),
        "An unrelated keys field must not be inventoried as membership."
    );
}

#[test]
fn canonical_origin_rejects_terminal_name_counterfeits() {
    let rejected = report("rejected");
    let root = rejected
        .roots
        .iter()
        .find(|root| root.name == "CounterfeitEngine")
        .expect("The counterfeit engine must be inventoried.");
    assert!(
        root.membership_fields.is_empty(),
        "Terminal type names must not establish canonical membership origin."
    );
    assert!(
        rejected.violations.iter().any(|violation| {
            violation.code == ViolationCode::ReimplementedCoreVocabulary
                && violation.message.contains("ItemRequirements")
        }),
        "The counterfeit vocabulary must be rejected as a local reimplementation."
    );
}

#[test]
fn counterfeit_core_package_name_is_rejected() {
    let counterfeit = check_repository_roots(
        &core_manifest(),
        &[fixture("counterfeit-core"), fixture("counterfeit-consumer")],
    )
    .expect("The counterfeit core fixtures must produce a report.");
    let root = counterfeit
        .roots
        .iter()
        .find(|root| root.name == "CounterfeitCoreRequirement")
        .expect("The counterfeit core requirement must be inventoried.");
    assert!(
        root.membership_fields.is_empty(),
        "A package name must not establish canonical core vocabulary."
    );
    let consumer = counterfeit
        .roots
        .iter()
        .find(|root| root.name == "CounterfeitDependencyRequirement")
        .expect("The counterfeit dependency requirement must be inventoried.");
    assert!(
        consumer.membership_fields.is_empty(),
        "A path dependency with the canonical package name must not establish core vocabulary."
    );
    assert!(
        counterfeit
            .violations
            .iter()
            .any(|violation| { violation.code == ViolationCode::ReimplementedCoreVocabulary }),
        "The counterfeit core package must be rejected as a local vocabulary copy."
    );
}

#[test]
fn shadowed_membership_transfer_is_rejected() {
    for function in [
        "shadowed_membership_transfer",
        "tuple_shadowed_membership_transfer",
    ] {
        let violations = violations_for_function(function);
        assert_eq!(
            violations.len(),
            1,
            "A later binding must replace the transfer provenance of the shadowed name."
        );
        assert!(violations[0].contains("constructs or obtains membership"));
    }
}

#[test]
fn canonical_origin_accepts_public_reexport_chain() {
    let accepted = report("accepted");
    let root = accepted
        .roots
        .iter()
        .find(|root| root.name == "AcceptedAdapterRequirement")
        .expect("The adapter requirement must be inventoried.");
    assert_eq!(
        root.membership_fields.len(),
        1,
        "A proven public core re-export must retain canonical membership identity."
    );
}

#[test]
fn canonical_origin_rejects_nested_counterfeit_from_mixed_facade() {
    let report = check_repository_roots(
        &core_manifest(),
        &[fixture("mixed-facade"), fixture("mixed-facade-consumer")],
    )
    .expect("The mixed facade fixtures must produce an architecture report.");
    let root = report
        .roots
        .iter()
        .find(|root| root.name == "CounterfeitNestedRequirement")
        .expect("The mixed facade consumer must be inventoried.");
    assert!(
        root.membership_fields.is_empty(),
        "A canonical facade must not authorize same-named types under unrelated nested modules."
    );
}

#[test]
fn canonical_origin_uses_dependency_manifest_identity() {
    let report = check_repository_roots(
        &core_manifest(),
        &[
            fixture("accepted/core"),
            fixture("canonical-facade"),
            fixture("duplicate-facade"),
            fixture("duplicate-facade-consumer"),
        ],
    )
    .expect("The duplicate facade fixtures must produce an architecture report.");
    let root = report
        .roots
        .iter()
        .find(|root| root.name == "DuplicateFacadeRequirement")
        .expect("The duplicate facade consumer must be inventoried.");
    assert!(
        root.membership_fields.is_empty(),
        "A path dependency must use its manifest identity, not another package with the same name."
    );
}

#[test]
fn macro_only_requirement_crate_is_rejected() {
    let report = report("macro-only");
    assert!(
        report.violations.iter().any(|violation| {
            violation.code == ViolationCode::UninspectableRequirementMacro
                && violation.message.contains("parameterized_requirement")
        }),
        "A parameterized macro that hides a requirement root must be rejected."
    );
}

#[test]
fn closure_membership_parameter_is_rejected() {
    let violations = violations_for_function("closure_membership_parameter");
    assert!(
        violations
            .iter()
            .any(|message| message.contains("accepts membership through a helper parameter")),
        "A closure must not accept membership through a helper parameter."
    );
    assert!(
        violations
            .iter()
            .any(|message| message.contains("mutates a membership collection")),
        "A closure must not mutate membership received through its parameter."
    );
}

#[test]
fn inferred_closure_membership_is_rejected() {
    let violations = violations_for_function("inferred_closure_membership");
    assert!(
        violations
            .iter()
            .any(|message| message.contains("passes membership through a local closure")),
        "Inferred closure parameters must not hide membership transfer."
    );
}

#[test]
fn tuple_bound_closure_alias_is_rejected() {
    let violations = violations_for_function("tuple_closure_alias_membership");
    assert!(
        violations
            .iter()
            .any(|message| message.contains("passes membership through a local closure")),
        "Tuple binding must preserve closure identity."
    );
}

#[test]
fn referenced_closure_alias_is_rejected() {
    let violations = violations_for_function("referenced_closure_alias_membership");
    assert!(
        violations
            .iter()
            .any(|message| message.contains("passes membership through a local closure")),
        "Referencing a closure must preserve closure identity."
    );
}

#[test]
fn imported_adapter_alias_is_tracked() {
    let violations = violations_for_function("imported_adapter_alias_discard");
    assert!(
        violations
            .iter()
            .any(|message| message.contains("constructs or obtains membership")),
        "A renamed adapter import must not hide discarded policy membership."
    );
}

#[test]
fn parent_module_trait_alias_is_inventoried() {
    let rejected = report("rejected");
    assert!(
        rejected
            .roots
            .iter()
            .any(|root| root.name == "ParentAliasedEngine"),
        "A parent-module requirement trait alias must not hide a nested root."
    );
}

#[test]
fn chained_parent_module_trait_alias_is_inventoried() {
    let rejected = report("rejected");
    assert!(
        rejected
            .roots
            .iter()
            .any(|root| root.name == "ChainedAliasedEngine"),
        "A chained parent-module requirement trait alias must not hide a nested root."
    );
}

#[test]
fn custom_target_and_path_module_are_inventoried() {
    let report = report("custom-source");
    assert!(
        report.violations.is_empty(),
        "The custom source fixture must pass."
    );
    for root in [
        "RootRequirement",
        "PathRequirement",
        "ExternalAliasRequirement",
    ] {
        assert!(
            report.roots.iter().any(|item| item.name == root),
            "The checker must inventory {root}."
        );
    }
}

#[test]
fn nested_inline_path_module_is_inventoried() {
    let report = report("custom-source");
    assert!(
        report
            .roots
            .iter()
            .any(|root| root.name == "NestedPathRequirement"),
        "A path module nested in an inline module must be inventoried."
    );
}

#[test]
fn local_same_name_type_shadows_ancestor_adapter_root() {
    let accepted = report("accepted");
    assert!(
        accepted.violations.is_empty(),
        "A local same-name type must not resolve to an adapter root in another module: {:?}",
        accepted.violations
    );
}

#[test]
fn generic_type_parameter_shadow_is_accepted() {
    let accepted = report("accepted");
    assert!(
        accepted.violations.is_empty(),
        "A generic parameter must not inherit same-terminal adapter identity: {:?}",
        accepted.violations
    );
}

#[test]
fn unrelated_same_terminal_output_is_accepted() {
    let accepted = report("accepted");
    assert!(
        accepted.violations.is_empty(),
        "An unrelated output struct must not inherit same-terminal engine identity: {:?}",
        accepted.violations
    );
}

#[test]
fn imported_semantic_membership_alias_is_inventoried() {
    let accepted = report("accepted");
    let root = accepted
        .roots
        .iter()
        .find(|root| root.name == "ImportedAliasEngine")
        .expect("The imported semantic alias root must be inventoried.");
    assert_eq!(root.membership_fields.len(), 1);
    assert_eq!(root.membership_fields[0].name, "setting_keys");
}

#[test]
fn glob_requirement_import_is_rejected() {
    let report = report("rejected");
    assert!(report.violations.iter().any(|violation| violation.code
        == ViolationCode::UninspectableRequirementImport
        && violation.message.contains('*')));
}

#[test]
fn block_local_requirement_import_is_rejected() {
    let report = report("rejected");
    assert!(report.violations.iter().any(|violation| violation.code
        == ViolationCode::UninspectableRequirementImport
        && violation.message.contains("impostor")));
}

#[test]
fn direct_transfer_typed_local_and_map_are_accepted() {
    let accepted = report("accepted");
    assert!(
        accepted.violations.is_empty(),
        "Direct field transfer, a typed direct-transfer local, and ItemRequirements::map must pass."
    );
}

#[test]
fn inventory_is_machine_readable_and_complete() {
    let report = check_repository_roots(&core_manifest(), &[fixture("inventory")])
        .expect("The inventory fixture must produce an architecture report.");
    assert!(
        report.violations.is_empty(),
        "The inventory fixture must pass."
    );
    assert_eq!(
        report.roots.len(),
        2,
        "Both requirement roots must be inventoried."
    );
    assert!(
        report
            .roots
            .iter()
            .any(|root| root.kind == RequirementKind::Engine),
        "The engine requirement root must be inventoried."
    );
    assert!(
        report
            .roots
            .iter()
            .any(|root| root.kind == RequirementKind::Adapter),
        "The adapter requirement root must be inventoried."
    );
    assert!(
        report
            .roots
            .iter()
            .all(|root| root.membership_fields.len() == 1),
        "Each root must expose one explicit membership field."
    );
    let json =
        serde_json::to_value(&report).expect("The architecture inventory must serialize as JSON.");
    assert!(
        json.get("roots").is_some(),
        "JSON inventory must contain roots."
    );
    assert!(
        json.get("violations").is_some(),
        "JSON inventory must contain violations."
    );
}

#[test]
fn imported_canonical_requirement_trait_is_inventoried() {
    let report = check_repository_roots(
        &core_manifest(),
        &[fixture("accepted/core"), fixture("accepted/engine")],
    )
    .expect("The cross-crate requirement fixture must produce an architecture report.");
    assert!(
        report.roots.iter().any(|root| {
            root.crate_name == "architecture-fixture-engine"
                && root.name == "ImportedEngineRequirements"
                && root.kind == RequirementKind::Engine
        }),
        "A requirement trait imported from its canonical provider must inventory the implementing root."
    );
}

#[test]
fn multi_hop_requirement_trait_reexport_is_inventoried() {
    let report = check_repository_roots(
        &core_manifest(),
        &[
            fixture("accepted/core"),
            fixture("requirement-facade-a"),
            fixture("requirement-facade-b"),
            fixture("requirement-facade-consumer"),
        ],
    )
    .expect("The multi-hop requirement fixture must produce an architecture report.");
    assert!(
        report.roots.iter().any(|root| {
            root.crate_name == "requirement-facade-consumer"
                && root.name == "MultiHopEngineRequirements"
                && root.kind == RequirementKind::Engine
        }),
        "Requirement trait identity must survive every public facade re-export."
    );
}
