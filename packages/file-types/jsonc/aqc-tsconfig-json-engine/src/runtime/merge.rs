use aqc_file_engine_core::{ConflictEntry, Provenance, resolve_map};

use crate::{ResolvedTsconfigJsonRequirements, TsconfigJsonRequirements};

impl TsconfigJsonRequirements {
    /// Merges compiler-option requirements while retaining every contributor.
    ///
    /// # Errors
    ///
    /// Returns every incompatible scalar assertion, keyed by its `TSConfig` path.
    pub fn merge(
        requirements: Vec<(Provenance, Self)>,
    ) -> Result<ResolvedTsconfigJsonRequirements, Vec<ConflictEntry>> {
        let mut conflicts = Vec::new();
        let boolean_compiler_options = resolve_map(
            requirements
                .into_iter()
                .map(|(provenance, requirement)| (provenance, requirement.boolean_compiler_options))
                .collect(),
            |option| format!("compilerOptions.{}", option.file_key()),
            &mut conflicts,
        );
        if conflicts.is_empty() {
            Ok(ResolvedTsconfigJsonRequirements {
                boolean_compiler_options,
            })
        } else {
            Err(conflicts)
        }
    }
}
