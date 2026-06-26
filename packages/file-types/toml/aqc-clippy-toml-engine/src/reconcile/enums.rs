//! Reconciliation for clippy.toml string-valued (enum-style) settings.

#![cfg_attr(
    not(test),
    expect(
        clippy::missing_docs_in_private_items,
        reason = "Private setting reconciliation helpers are internal reconciliation steps."
    )
)]
#![expect(
    clippy::type_complexity,
    reason = "Private setting reconciliation helpers carry repeated resolved requirement shapes."
)]

use std::collections::BTreeMap;

use aqc_file_engine_core::{
    ConfigScalar, Finding, Provenance, ResolvedRequirement, ScalarAssertion,
};
use aqc_toml_engine_core::{apply_scalar_assertion, scalar_assertion_fails};
use toml_edit::DocumentMut;

/// Apply every string-setting requirement.
pub(crate) fn apply(
    doc: &mut DocumentMut,
    merged_by_key: &BTreeMap<
        String,
        ResolvedRequirement<ScalarAssertion<String>, ScalarAssertion<String>>,
    >,
    findings: &mut Vec<Finding>,
) {
    for (key, merged) in merged_by_key {
        let attribution = attribution_for(doc, key, merged);
        apply_one(doc, key, &merged.merged, &attribution, findings);
    }
}

fn attribution_for(
    doc: &DocumentMut,
    key: &str,
    resolved: &ResolvedRequirement<ScalarAssertion<String>, ScalarAssertion<String>>,
) -> Vec<Provenance> {
    let current = doc.get(key);
    let filtered = resolved
        .collected
        .iter()
        .filter(|(_, assertion)| {
            scalar_assertion_fails(current, &string_assertion_to_config(assertion))
        })
        .map(|(prov, _)| prov.clone())
        .collect::<Vec<_>>();
    if filtered.is_empty() {
        resolved
            .collected
            .iter()
            .map(|(prov, _)| prov.clone())
            .collect()
    } else {
        filtered
    }
}

/// Apply a single scalar assertion against a string setting.
fn apply_one(
    doc: &mut DocumentMut,
    key: &str,
    assertion: &ScalarAssertion<String>,
    attribution: &[Provenance],
    findings: &mut Vec<Finding>,
) {
    apply_scalar_assertion(
        doc,
        key,
        &string_assertion_to_config(assertion),
        attribution,
        findings,
    );
}

fn string_assertion_to_config(
    assertion: &ScalarAssertion<String>,
) -> ScalarAssertion<ConfigScalar> {
    match assertion {
        ScalarAssertion::Equals(value, msg) => {
            ScalarAssertion::Equals(ConfigScalar::Str(value.clone()), msg.clone())
        }
        ScalarAssertion::AtLeast(value, msg) => {
            ScalarAssertion::AtLeast(ConfigScalar::Str(value.clone()), msg.clone())
        }
        ScalarAssertion::AtMost(value, msg) => {
            ScalarAssertion::AtMost(ConfigScalar::Str(value.clone()), msg.clone())
        }
        ScalarAssertion::Range(min, max, msg) => ScalarAssertion::Range(
            ConfigScalar::Str(min.clone()),
            ConfigScalar::Str(max.clone()),
            msg.clone(),
        ),
        ScalarAssertion::OneOf(values, msg) => ScalarAssertion::OneOf(
            values
                .iter()
                .map(|value| ConfigScalar::Str(value.clone()))
                .collect(),
            msg.clone(),
        ),
        ScalarAssertion::Present(msg) => ScalarAssertion::Present(msg.clone()),
        ScalarAssertion::Absent(msg) => ScalarAssertion::Absent(msg.clone()),
    }
}
