use aqc_file_engine_core::merge::ResolvedMap;
use aqc_file_engine_core::{
    ConfigScalar, Finding, ResolvedRequirement, ScalarAssertion, Severity, resolved_map_attribution,
};
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
    reconcile_string_map(document, "scripts", requirement.scripts(), findings);
    reconcile_string_map(
        document,
        "devDependencies",
        requirement.dev_dependencies(),
        findings,
    );
}

fn reconcile_string_map(
    document: &mut JsonObject,
    parent: &str,
    requirements: &ResolvedMap<String, ScalarAssertion<String>>,
    findings: &mut Vec<Finding>,
) {
    if requirements.is_empty() {
        return;
    }
    if document.value_exists(&[parent]) && !document.object_exists(&[parent]) {
        findings.push(Finding::Mismatch {
            key: parent.to_owned(),
            selector: None,
            current: document.rendered_value(&[parent]),
            expected: "object".to_owned(),
            message: format!("{parent} must be an object."),
            severity: Severity::Error,
            attribution: resolved_map_attribution(requirements),
        });
        return;
    }
    for (key, resolved) in requirements {
        let findings_before = findings.len();
        reconcile_scalar_assertion(
            document,
            &[parent, key],
            resolved,
            |value| Some(ConfigScalar::Str(value.to_owned())),
            |scalar| match scalar {
                ConfigScalar::Str(value) => Some(value),
                ConfigScalar::Int(_) | ConfigScalar::Bool(_) => None,
            },
            findings,
        );
        for finding in findings.iter_mut().skip(findings_before) {
            if let Finding::Mismatch { selector, .. } = finding {
                *selector = Some(key.to_owned());
            }
        }
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
