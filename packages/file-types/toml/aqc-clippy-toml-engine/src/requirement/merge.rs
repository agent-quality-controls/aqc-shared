//! Clippy requirement merge orchestration.

#![expect(
    clippy::needless_pass_by_value,
    clippy::too_many_lines,
    clippy::type_complexity,
    clippy::use_self,
    reason = "Merge orchestration is a field-by-field aggregate construction."
)]

use std::collections::BTreeMap;

use aqc_file_engine_core::{
    ConflictEntry, DottedVersion, Provenance, Resolve, ResolvedRequirement, ScalarAssertion,
    resolve_forbidden_globs, resolve_items,
};

use super::{
    ClippyTomlRequirements, ResolvedClippyTomlRequirements, bans::push_clippy_path_glob_conflicts,
};

impl ClippyTomlRequirements {
    #[must_use]
    pub fn merge(
        reqs: Vec<(Provenance, ClippyTomlRequirements)>,
    ) -> (ResolvedClippyTomlRequirements, Vec<ConflictEntry>) {
        let mut conflicts = Vec::new();
        let disallowed_methods = resolve_items(
            "disallowed-methods",
            reqs.iter()
                .map(|(prov, req)| (prov.clone(), req.disallowed_methods.clone()))
                .collect(),
            &mut conflicts,
        );
        let forbidden_disallowed_method_path_globs = resolve_forbidden_globs(
            "disallowed-methods",
            reqs.iter()
                .map(|(prov, req)| {
                    (
                        prov.clone(),
                        req.forbidden_disallowed_method_path_globs.clone(),
                    )
                })
                .collect(),
            &mut conflicts,
        );
        let disallowed_method_glob_conflicts = push_clippy_path_glob_conflicts(
            "disallowed-methods",
            "disallowed-method-path-glob-forbids-required-path",
            &disallowed_methods,
            &forbidden_disallowed_method_path_globs,
            &mut conflicts,
        );

        let disallowed_types = resolve_items(
            "disallowed-types",
            reqs.iter()
                .map(|(prov, req)| (prov.clone(), req.disallowed_types.clone()))
                .collect(),
            &mut conflicts,
        );
        let forbidden_disallowed_type_path_globs = resolve_forbidden_globs(
            "disallowed-types",
            reqs.iter()
                .map(|(prov, req)| {
                    (
                        prov.clone(),
                        req.forbidden_disallowed_type_path_globs.clone(),
                    )
                })
                .collect(),
            &mut conflicts,
        );
        let disallowed_type_glob_conflicts = push_clippy_path_glob_conflicts(
            "disallowed-types",
            "disallowed-type-path-glob-forbids-required-path",
            &disallowed_types,
            &forbidden_disallowed_type_path_globs,
            &mut conflicts,
        );

        let disallowed_macros = resolve_items(
            "disallowed-macros",
            reqs.iter()
                .map(|(prov, req)| (prov.clone(), req.disallowed_macros.clone()))
                .collect(),
            &mut conflicts,
        );
        let forbidden_disallowed_macro_path_globs = resolve_forbidden_globs(
            "disallowed-macros",
            reqs.iter()
                .map(|(prov, req)| {
                    (
                        prov.clone(),
                        req.forbidden_disallowed_macro_path_globs.clone(),
                    )
                })
                .collect(),
            &mut conflicts,
        );
        let disallowed_macro_glob_conflicts = push_clippy_path_glob_conflicts(
            "disallowed-macros",
            "disallowed-macro-path-glob-forbids-required-path",
            &disallowed_macros,
            &forbidden_disallowed_macro_path_globs,
            &mut conflicts,
        );

        let out = ResolvedClippyTomlRequirements {
            msrv: resolve_validated_optional(
                "msrv",
                reqs.iter()
                    .map(|(prov, req)| (prov.clone(), req.msrv.clone()))
                    .collect(),
                clippy_msrv_assertion_is_legal,
                &mut conflicts,
            ),
            thresholds: resolve_validated_map(
                reqs.iter()
                    .map(|(prov, req)| (prov.clone(), req.thresholds.clone()))
                    .collect(),
                Clone::clone,
                clippy_threshold_assertion_is_legal,
                &mut conflicts,
            ),
            disallowed_methods,
            forbidden_disallowed_method_path_globs,
            disallowed_method_glob_conflicts,
            disallowed_types,
            forbidden_disallowed_type_path_globs,
            disallowed_type_glob_conflicts,
            disallowed_macros,
            forbidden_disallowed_macro_path_globs,
            disallowed_macro_glob_conflicts,
            bools: resolve_validated_map(
                reqs.iter()
                    .map(|(prov, req)| (prov.clone(), req.bools.clone()))
                    .collect(),
                Clone::clone,
                clippy_bool_assertion_is_legal,
                &mut conflicts,
            ),
            enums: resolve_validated_map(
                reqs.iter()
                    .map(|(prov, req)| (prov.clone(), req.enums.clone()))
                    .collect(),
                Clone::clone,
                clippy_enum_assertion_is_legal,
                &mut conflicts,
            ),
        };
        (out, conflicts)
    }
}

fn resolve_validated_optional<A>(
    key: &str,
    input: Vec<(Provenance, Option<A>)>,
    is_legal: impl Fn(&A) -> bool,
    conflicts: &mut Vec<ConflictEntry>,
) -> Option<ResolvedRequirement<A::Merged, A>>
where
    A: Resolve + core::fmt::Debug,
{
    let items = input
        .into_iter()
        .filter_map(|(prov, assertion)| assertion.map(|assertion| (prov, assertion)))
        .collect::<Vec<_>>();
    if items.is_empty() {
        return None;
    }
    if !items.iter().all(|(_, assertion)| is_legal(assertion)) {
        push_unsupported_conflict(key, &items, conflicts);
        return None;
    }
    A::resolve(key, items, conflicts)
}

fn resolve_validated_map<K, A>(
    input: Vec<(Provenance, BTreeMap<K, A>)>,
    key_path: impl Fn(&K) -> String,
    is_legal: impl Fn(&A) -> bool,
    conflicts: &mut Vec<ConflictEntry>,
) -> BTreeMap<K, ResolvedRequirement<A::Merged, A>>
where
    K: Ord + Clone,
    A: Resolve + core::fmt::Debug,
{
    let mut by_key: BTreeMap<K, Vec<(Provenance, A)>> = BTreeMap::new();
    for (prov, map) in input {
        for (key, assertion) in map {
            by_key
                .entry(key)
                .or_default()
                .push((prov.clone(), assertion));
        }
    }

    let mut out = BTreeMap::new();
    for (key, items) in by_key {
        let key_text = key_path(&key);
        if !items.iter().all(|(_, assertion)| is_legal(assertion)) {
            push_unsupported_conflict(&key_text, &items, conflicts);
            continue;
        }
        if let Some(resolved) = A::resolve(&key_text, items, conflicts) {
            let _ = out.insert(key, resolved);
        }
    }
    out
}

fn clippy_msrv_assertion_is_legal(assertion: &ScalarAssertion<DottedVersion>) -> bool {
    matches!(
        assertion,
        ScalarAssertion::Equals(..)
            | ScalarAssertion::AtLeast(..)
            | ScalarAssertion::OneOf(..)
            | ScalarAssertion::Present(_)
            | ScalarAssertion::Absent(_)
    )
}

fn clippy_threshold_assertion_is_legal(assertion: &ScalarAssertion<u64>) -> bool {
    matches!(
        assertion,
        ScalarAssertion::Equals(..)
            | ScalarAssertion::AtMost(..)
            | ScalarAssertion::AtLeast(..)
            | ScalarAssertion::Range(..)
            | ScalarAssertion::Present(_)
            | ScalarAssertion::Absent(_)
    )
}

fn clippy_bool_assertion_is_legal(assertion: &ScalarAssertion<bool>) -> bool {
    matches!(
        assertion,
        ScalarAssertion::Equals(..) | ScalarAssertion::Present(_) | ScalarAssertion::Absent(_)
    )
}

fn clippy_enum_assertion_is_legal(assertion: &ScalarAssertion<String>) -> bool {
    matches!(
        assertion,
        ScalarAssertion::Equals(..)
            | ScalarAssertion::OneOf(..)
            | ScalarAssertion::Present(_)
            | ScalarAssertion::Absent(_)
    )
}

fn push_unsupported_conflict<A: core::fmt::Debug>(
    key: &str,
    items: &[(Provenance, A)],
    conflicts: &mut Vec<ConflictEntry>,
) {
    conflicts.push(ConflictEntry {
        key: key.to_owned(),
        reason: "scalar-operation-unsupported".to_owned(),
        contributors: items
            .iter()
            .map(|(prov, assertion)| (prov.clone(), format!("{assertion:?}")))
            .collect(),
    });
}
