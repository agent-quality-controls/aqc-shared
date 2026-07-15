use aqc_file_engine_core::{
    ConflictEntry, resolve_forbidden_globs, resolve_items, resolve_list, resolve_map,
};

use super::collect::{MergeInputs, RequirementContributions};
use super::conflicts::report_structural_conflicts;
use super::required_globs::report_required_glob_conflicts;
use crate::types::{JsonFileRequirements, JsonPath, ResolvedJsonFileRequirements};

type MergeResult = Result<ResolvedJsonFileRequirements, Vec<ConflictEntry>>;

impl JsonFileRequirements {
    /// Compose all policy contributions into one reconciler input.
    ///
    /// # Errors
    ///
    /// Returns every incompatible requirement with contributor attribution.
    pub fn merge(requirements: RequirementContributions) -> MergeResult {
        let inputs = MergeInputs::collect(requirements);
        let mut conflicts = Vec::new();
        report_structural_conflicts(&inputs, &mut conflicts);

        let scalar_values =
            resolve_map(inputs.scalar_values, JsonPath::finding_key, &mut conflicts);
        let string_lists = inputs
            .string_lists
            .into_iter()
            .map(|(path, inputs)| {
                let resolved = resolve_list(&path, inputs, &mut conflicts);
                (path, resolved)
            })
            .collect();
        let forbidden_string_list_globs = inputs
            .forbidden_string_list_globs
            .into_iter()
            .map(|(path, inputs)| {
                let resolved = resolve_forbidden_globs(&path.finding_key(), inputs, &mut conflicts);
                (path, resolved)
            })
            .collect();
        let object_keys = inputs
            .object_keys
            .into_iter()
            .map(|(path, inputs)| {
                let resolved = resolve_items(&path, inputs, &mut conflicts);
                (path, resolved)
            })
            .collect();
        report_required_glob_conflicts(&string_lists, &forbidden_string_list_globs, &mut conflicts);

        if conflicts.is_empty() {
            Ok(ResolvedJsonFileRequirements {
                scalar_values,
                string_lists,
                forbidden_string_list_globs,
                object_keys,
            })
        } else {
            Err(conflicts)
        }
    }
}
