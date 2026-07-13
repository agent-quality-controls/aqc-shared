//! Clippy requirement merge orchestration.

#![allow(
    clippy::needless_pass_by_value,
    clippy::too_many_lines,
    clippy::type_complexity,
    clippy::use_self,
    reason = "Merge orchestration is a field-by-field aggregate construction."
)]

use std::collections::BTreeMap;

use aqc_file_engine_core::{
    ConflictEntry, DottedVersion, Provenance, Resolve, ResolvedRequirement, ScalarAssertion,
    push_conflict, render_scalar_assertion, resolve_forbidden_globs, resolve_items,
};

use super::{
    ClippyTomlRequirements, ResolvedClippyTomlRequirements,
    disallowed::push_clippy_path_glob_conflicts,
};

impl ClippyTomlRequirements {
    /// Merges all Clippy TOML requirements into one resolved requirement set.
    ///
    /// # Errors
    ///
    /// Returns every conflict when the input requirements cannot be composed.
    pub fn merge(
        reqs: Vec<(Provenance, ClippyTomlRequirements)>,
    ) -> Result<ResolvedClippyTomlRequirements, Vec<ConflictEntry>> {
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
        push_clippy_path_glob_conflicts(
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
        push_clippy_path_glob_conflicts(
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
        push_clippy_path_glob_conflicts(
            "disallowed-macros",
            "disallowed-macro-path-glob-forbids-required-path",
            &disallowed_macros,
            &forbidden_disallowed_macro_path_globs,
            &mut conflicts,
        );

        let resolved = ResolvedClippyTomlRequirements {
            msrv: resolve_validated_optional(
                "msrv",
                reqs.iter()
                    .map(|(prov, req)| (prov.clone(), req.msrv.clone()))
                    .collect(),
                clippy_msrv_assertion_is_legal,
                render_scalar_assertion,
                &mut conflicts,
            ),
            thresholds: resolve_validated_map(
                reqs.iter()
                    .map(|(prov, req)| (prov.clone(), req.thresholds.clone()))
                    .collect(),
                Clone::clone,
                clippy_threshold_assertion_is_legal,
                render_scalar_assertion,
                &mut conflicts,
            ),
            disallowed_methods,
            forbidden_disallowed_method_path_globs,
            disallowed_types,
            forbidden_disallowed_type_path_globs,
            disallowed_macros,
            forbidden_disallowed_macro_path_globs,
            bools: resolve_validated_map(
                reqs.iter()
                    .map(|(prov, req)| (prov.clone(), req.bools.clone()))
                    .collect(),
                Clone::clone,
                clippy_bool_assertion_is_legal,
                render_scalar_assertion,
                &mut conflicts,
            ),
            enums: resolve_validated_map(
                reqs.iter()
                    .map(|(prov, req)| (prov.clone(), req.enums.clone()))
                    .collect(),
                Clone::clone,
                clippy_enum_assertion_is_legal,
                render_scalar_assertion,
                &mut conflicts,
            ),
        };
        if conflicts.is_empty() {
            Ok(resolved)
        } else {
            Err(conflicts)
        }
    }
}

/// Resolves an optional scalar assertion after checking engine-specific legality.
fn resolve_validated_optional<A>(
    key: &str,
    input: Vec<(Provenance, Option<A>)>,
    is_legal: impl Fn(&A) -> bool,
    render: impl Fn(&A) -> String,
    conflicts: &mut Vec<ConflictEntry>,
) -> Option<ResolvedRequirement<A::Merged, A>>
where
    A: Resolve,
{
    let items = input
        .into_iter()
        .filter_map(|(prov, assertion)| assertion.map(|assertion| (prov, assertion)))
        .collect::<Vec<_>>();
    if items.is_empty() {
        return None;
    }
    if !items.iter().all(|(_, assertion)| is_legal(assertion)) {
        push_conflict(
            key,
            "scalar-operation-unsupported",
            &items,
            render,
            conflicts,
        );
        return None;
    }
    A::resolve(key, items, conflicts)
}

/// Resolves a map of scalar assertions after checking engine-specific legality.
fn resolve_validated_map<K, A>(
    input: Vec<(Provenance, BTreeMap<K, A>)>,
    key_path: impl Fn(&K) -> String,
    is_legal: impl Fn(&A) -> bool,
    render: impl Fn(&A) -> String,
    conflicts: &mut Vec<ConflictEntry>,
) -> BTreeMap<K, ResolvedRequirement<A::Merged, A>>
where
    K: Ord + Clone,
    A: Resolve,
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
            push_conflict(
                &key_text,
                "scalar-operation-unsupported",
                &items,
                &render,
                conflicts,
            );
            continue;
        }
        if let Some(resolved) = A::resolve(&key_text, items, conflicts) {
            let _ = out.insert(key, resolved);
        }
    }
    out
}

/// Returns whether an `msrv` assertion can be represented in `clippy.toml`.
const fn clippy_msrv_assertion_is_legal(assertion: &ScalarAssertion<DottedVersion>) -> bool {
    matches!(
        assertion,
        ScalarAssertion::Equals(..)
            | ScalarAssertion::AtLeast(..)
            | ScalarAssertion::OneOf(..)
            | ScalarAssertion::Present(_)
            | ScalarAssertion::Absent(_)
    )
}

/// Returns whether a numeric threshold assertion can be represented in `clippy.toml`.
const fn clippy_threshold_assertion_is_legal(assertion: &ScalarAssertion<u64>) -> bool {
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

/// Returns whether a boolean assertion can be represented in `clippy.toml`.
const fn clippy_bool_assertion_is_legal(assertion: &ScalarAssertion<bool>) -> bool {
    matches!(
        assertion,
        ScalarAssertion::Equals(..) | ScalarAssertion::Present(_) | ScalarAssertion::Absent(_)
    )
}

/// Returns whether an enum-like string assertion can be represented in `clippy.toml`.
const fn clippy_enum_assertion_is_legal(assertion: &ScalarAssertion<String>) -> bool {
    matches!(
        assertion,
        ScalarAssertion::Equals(..)
            | ScalarAssertion::OneOf(..)
            | ScalarAssertion::Present(_)
            | ScalarAssertion::Absent(_)
    )
}
