use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use aqc_requirement_architecture::{RequirementKind, ViolationCode, check_repository_roots};

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

#[test]
fn rejected_fixture_covers_membership_construction_and_disguised_roots() {
    let report = check_repository_roots(&[fixture("rejected")])
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
    for expected in [
        "constructs ItemRequirements",
        "macro",
        "mutating or unrecognized method",
        "borrows a membership collection mutably",
        "destructures membership internals",
        "PrivateClosureField field exact_settings",
        "PrivateNestedClosure field exact_settings",
        "reimplements core vocabulary type ItemRequirements",
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
    for function in [
        "direct_exact",
        "nested_keys",
        "inferred_values",
        "optional_filter",
        "represented_discovery",
        "assigned_exact",
        "renamed_local_membership_mutation",
        "hidden_default_construction",
        "cross_crate_membership_helper",
        "cross_crate_wrapper_field",
        "cross_crate_engine_membership_helper",
        "rewrite_membership_parameter",
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
    ] {
        assert!(
            messages.contains(&format!("function {function}")),
            "The adversarial case {function} must produce its own violation."
        );
    }
    assert!(
        report.roots.iter().any(|root| root.name == "HiddenAdapter"),
        "Renamed AdapterRequirement imports must not hide requirement roots."
    );
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
    ] {
        assert!(codes.contains(&code), "The checker must reject {purpose}.");
    }
}

#[test]
fn policy_construction_and_adapter_map_are_accepted() {
    let report = check_repository_roots(&[fixture("accepted")])
        .expect("The accepted fixtures must produce an architecture report.");
    assert!(
        report.violations.is_empty(),
        "Policy construction, imported_membership_alias, adapter map, neutral engine defaults, unrelated field mutations, and unrelated macros must pass: {:?}",
        report.violations
    );
}

#[test]
fn inventory_is_machine_readable_and_complete() {
    let report = check_repository_roots(&[fixture("inventory")])
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
