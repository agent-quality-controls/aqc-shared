//! Scalar setting reconciliation.

use aqc_file_engine_core::{ConfigScalar, Finding, Provenance, ScalarAssertion};
use toml_edit::DocumentMut;

/// Applies one resolved scalar assertion.
pub(super) fn apply_scalar(
    doc: &mut DocumentMut,
    key: &str,
    assertion: &ScalarAssertion<ConfigScalar>,
    attribution: &[Provenance],
    findings: &mut Vec<Finding>,
) {
    aqc_toml_engine_core::apply_scalar_assertion(doc, key, assertion, attribution, findings);
}
