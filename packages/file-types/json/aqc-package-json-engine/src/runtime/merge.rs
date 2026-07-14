use aqc_file_engine_core::{ConflictEntry, Provenance, ScalarAssertion, resolve_maybe};

use crate::types::{
    PackageJsonRequirements, ResolvedDevEnginePackageManagerRequirements,
    ResolvedPackageJsonRequirements,
};

type RequirementInput = Vec<(Provenance, PackageJsonRequirements)>;

impl PackageJsonRequirements {
    /// Merges package JSON requirements while retaining every contributor.
    ///
    /// # Errors
    ///
    /// Returns every scalar conflict across the package-manager declarations.
    #[expect(
        clippy::needless_pass_by_value,
        reason = "merged_reconcile requires a merge callback that owns its routed requirement vector."
    )]
    pub fn merge(
        requirements: RequirementInput,
    ) -> Result<ResolvedPackageJsonRequirements, Vec<ConflictEntry>> {
        let mut conflicts = Vec::new();
        let resolved = ResolvedPackageJsonRequirements {
            package_manager: scalar(
                "packageManager",
                &requirements,
                |requirement| requirement.package_manager.clone(),
                &mut conflicts,
            ),
            dev_engines_package_manager: ResolvedDevEnginePackageManagerRequirements {
                name: scalar(
                    "devEngines.packageManager.name",
                    &requirements,
                    |requirement| requirement.dev_engines_package_manager.name.clone(),
                    &mut conflicts,
                ),
                version: scalar(
                    "devEngines.packageManager.version",
                    &requirements,
                    |requirement| requirement.dev_engines_package_manager.version.clone(),
                    &mut conflicts,
                ),
                on_fail: scalar(
                    "devEngines.packageManager.onFail",
                    &requirements,
                    |requirement| requirement.dev_engines_package_manager.on_fail.clone(),
                    &mut conflicts,
                ),
            },
        };
        if conflicts.is_empty() {
            Ok(resolved)
        } else {
            Err(conflicts)
        }
    }
}

fn scalar<T>(
    key: &str,
    requirements: &[(Provenance, PackageJsonRequirements)],
    field: impl Fn(&PackageJsonRequirements) -> Option<ScalarAssertion<T>>,
    conflicts: &mut Vec<ConflictEntry>,
) -> Option<aqc_file_engine_core::ResolvedRequirement<ScalarAssertion<T>, ScalarAssertion<T>>>
where
    T: aqc_file_engine_core::ScalarValue,
{
    resolve_maybe(
        key,
        requirements
            .iter()
            .map(|(provenance, requirement)| (provenance.clone(), field(requirement)))
            .collect(),
        conflicts,
    )
}
