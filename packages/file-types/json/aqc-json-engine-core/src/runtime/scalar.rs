use aqc_file_engine_core::{
    ConfigScalar, Finding, ResolvedRequirement, ScalarAssertion, ScalarValue, Severity,
    render_scalar_assertion, scalar_assertion_matches, scalar_assertion_writable_value,
};

use crate::{JsonObject, NonObjectParentAction};

#[expect(
    clippy::too_many_arguments,
    reason = "scalar reconciliation needs the document, field identity, write policy, assertion, codec, and finding sink"
)]
pub fn reconcile_scalar_assertion<T: ScalarValue>(
    document: &mut JsonObject,
    path: &[&str],
    selector: Option<String>,
    parent_action: NonObjectParentAction,
    requirement: &ResolvedRequirement<ScalarAssertion<T>, ScalarAssertion<T>>,
    encode: impl Fn(&T) -> Option<ConfigScalar>,
    decode: impl Fn(ConfigScalar) -> Option<T>,
    findings: &mut Vec<Finding>,
) {
    let exists = document.value_exists(path);
    let current = document.scalar(path).and_then(decode);
    if scalar_assertion_matches(&requirement.merged, current.as_ref(), exists) {
        return;
    }
    findings.push(Finding::Mismatch {
        key: path.join("."),
        selector,
        current: document.rendered_value(path),
        expected: render_scalar_assertion(&requirement.merged),
        message: requirement.merged.message().to_owned(),
        severity: Severity::Error,
        attribution: requirement.attribution(),
    });
    if let Some(value) = scalar_assertion_writable_value(&requirement.merged).and_then(encode) {
        let _ = document.set_scalar(path, value, parent_action);
    } else if matches!(requirement.merged, ScalarAssertion::Absent(_)) {
        let _ = document.remove_value(path);
    }
}
