use aqc_file_engine_core::{ConfigScalar, Finding, ResolvedRequirement, ScalarAssertion};
use aqc_json_engine_core::{JsonObject, reconcile_scalar_assertion};

use crate::types::{PackageManagerOnFail, ResolvedPackageJsonRequirements};

pub(super) fn reconcile_document(
    document: &mut JsonObject,
    requirement: &ResolvedPackageJsonRequirements,
    findings: &mut Vec<Finding>,
) {
    if let Some(resolved) = requirement.package_manager() {
        reconcile_scalar_assertion(
            document,
            &["packageManager"],
            resolved,
            |value| Some(ConfigScalar::Str(value.to_owned())),
            |scalar| match scalar {
                ConfigScalar::Str(value) => Some(value),
                ConfigScalar::Int(_) | ConfigScalar::Bool(_) => None,
            },
            findings,
        );
    }
    let nested = requirement.dev_engines_package_manager();
    if let Some(resolved) = nested.name() {
        reconcile_string(
            document,
            &["devEngines", "packageManager", "name"],
            resolved,
            findings,
        );
    }
    if let Some(resolved) = nested.version() {
        reconcile_string(
            document,
            &["devEngines", "packageManager", "version"],
            resolved,
            findings,
        );
    }
    if let Some(resolved) = nested.on_fail() {
        reconcile_scalar_assertion(
            document,
            &["devEngines", "packageManager", "onFail"],
            resolved,
            |value| Some(ConfigScalar::Str(value.as_str().to_owned())),
            |scalar| match scalar {
                ConfigScalar::Str(value) => PackageManagerOnFail::parse(&value),
                ConfigScalar::Int(_) | ConfigScalar::Bool(_) => None,
            },
            findings,
        );
    }
}

fn reconcile_string(
    document: &mut JsonObject,
    path: &[&str],
    resolved: &ResolvedRequirement<ScalarAssertion<String>, ScalarAssertion<String>>,
    findings: &mut Vec<Finding>,
) {
    reconcile_scalar_assertion(
        document,
        path,
        resolved,
        |value| Some(ConfigScalar::Str(value.to_owned())),
        |scalar| match scalar {
            ConfigScalar::Str(value) => Some(value),
            ConfigScalar::Int(_) | ConfigScalar::Bool(_) => None,
        },
        findings,
    );
}
