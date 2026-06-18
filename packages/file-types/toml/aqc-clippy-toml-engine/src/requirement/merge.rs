//! Clippy requirement merge orchestration.

use aqc_file_engine_core::{
    ConflictEntry, Provenance, resolve_forbidden_globs, resolve_items, resolve_map, resolve_maybe,
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
            msrv: resolve_maybe(
                "msrv",
                reqs.iter()
                    .map(|(prov, req)| (prov.clone(), req.msrv.clone()))
                    .collect(),
                &mut conflicts,
            ),
            thresholds: resolve_map(
                reqs.iter()
                    .map(|(prov, req)| (prov.clone(), req.thresholds.clone()))
                    .collect(),
                Clone::clone,
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
            bools: resolve_map(
                reqs.iter()
                    .map(|(prov, req)| (prov.clone(), req.bools.clone()))
                    .collect(),
                Clone::clone,
                &mut conflicts,
            ),
            enums: resolve_map(
                reqs.iter()
                    .map(|(prov, req)| (prov.clone(), req.enums.clone()))
                    .collect(),
                Clone::clone,
                &mut conflicts,
            ),
        };
        (out, conflicts)
    }
}
