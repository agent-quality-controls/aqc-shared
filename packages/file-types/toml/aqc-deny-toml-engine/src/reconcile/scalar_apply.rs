//! Generic scalar field application for deny.toml.

use core::cmp::Ordering;

use aqc_file_engine_core::{Finding, Provenance, ResolvedRequirement, ScalarAssertion, Severity};
use aqc_toml_engine_core::ScalarFieldEdit;
use toml_edit::{DocumentMut, Item};

use super::scalar_value::DenyTomlScalar;
use super::support::{ensure_table_path, render_item, table_item, table_path_mut};

type ResolvedScalar<T> = Option<ResolvedRequirement<ScalarAssertion<T>, ScalarAssertion<T>>>;

pub(super) fn apply_scalar<T>(
    doc: &mut DocumentMut,
    table_path: &[&str],
    field_key: &str,
    display_key: &str,
    requirement: &ResolvedScalar<T>,
    findings: &mut Vec<Finding>,
) where
    T: DenyTomlScalar,
{
    let Some(resolved) = requirement else {
        return;
    };
    let attribution = resolved.attribution();
    let current = table_item(doc, table_path, field_key);
    match scalar_edit(
        display_key,
        current,
        &resolved.merged,
        &attribution,
        findings,
    ) {
        Some(ScalarFieldEdit::Write(item)) => {
            let table = ensure_table_path(doc, table_path);
            table[field_key] = item;
        }
        Some(ScalarFieldEdit::Remove) => {
            if let Some(table) = table_path_mut(doc, table_path) {
                let _ = table.remove(field_key);
            }
        }
        None => {}
    }
}

fn scalar_edit<T>(
    display_key: &str,
    current: Option<&Item>,
    assertion: &ScalarAssertion<T>,
    attribution: &[Provenance],
    findings: &mut Vec<Finding>,
) -> Option<ScalarFieldEdit>
where
    T: DenyTomlScalar,
{
    match assertion {
        ScalarAssertion::Equals(want, message) => {
            if current.and_then(T::parse_item).as_ref() == Some(want) {
                return None;
            }
            findings.push(Finding::Mismatch {
                key: display_key.to_owned(),
                selector: None,
                current: current.map(render_item),
                expected: T::render_value(want),
                message: message.clone(),
                severity: Severity::Error,
                attribution: attribution.to_vec(),
            });
            Some(ScalarFieldEdit::Write(T::write_item(want)))
        }
        ScalarAssertion::OneOf(allowed, message) => {
            if current
                .and_then(T::parse_item)
                .is_some_and(|value| allowed.contains(&value))
            {
                return None;
            }
            findings.push(Finding::Mismatch {
                key: display_key.to_owned(),
                selector: None,
                current: current.map(render_item),
                expected: format!(
                    "one of {:?}",
                    allowed.iter().map(T::render_value).collect::<Vec<_>>()
                ),
                message: message.clone(),
                severity: Severity::Error,
                attribution: attribution.to_vec(),
            });
            None
        }
        ScalarAssertion::Present(message) => {
            if current.is_some() {
                return None;
            }
            findings.push(Finding::Mismatch {
                key: display_key.to_owned(),
                selector: None,
                current: None,
                expected: "present".to_owned(),
                message: message.clone(),
                severity: Severity::Error,
                attribution: attribution.to_vec(),
            });
            None
        }
        ScalarAssertion::Absent(message) => {
            let current = current.map(render_item)?;
            findings.push(Finding::Mismatch {
                key: display_key.to_owned(),
                selector: None,
                current: Some(current),
                expected: "absent".to_owned(),
                message: message.clone(),
                severity: Severity::Error,
                attribution: attribution.to_vec(),
            });
            Some(ScalarFieldEdit::Remove)
        }
        ScalarAssertion::AtLeast(want, message) => scalar_order_edit(
            display_key,
            current,
            want,
            message,
            Ordering::Less,
            attribution,
            findings,
        ),
        ScalarAssertion::AtMost(want, message) => scalar_order_edit(
            display_key,
            current,
            want,
            message,
            Ordering::Greater,
            attribution,
            findings,
        ),
        ScalarAssertion::Range(min, max, message) => {
            let current_value = current.and_then(T::parse_item);
            let in_range = current_value.as_ref().is_some_and(|value| {
                !matches!(value.compare_for_order(min), Some(Ordering::Less))
                    && !matches!(value.compare_for_order(max), Some(Ordering::Greater))
            });
            if in_range {
                return None;
            }
            findings.push(Finding::Mismatch {
                key: display_key.to_owned(),
                selector: None,
                current: current.map(render_item),
                expected: format!(
                    "between {} and {}",
                    T::render_value(min),
                    T::render_value(max)
                ),
                message: message.clone(),
                severity: Severity::Error,
                attribution: attribution.to_vec(),
            });
            Some(ScalarFieldEdit::Write(T::write_item(min)))
        }
    }
}

fn scalar_order_edit<T>(
    display_key: &str,
    current: Option<&Item>,
    want: &T,
    message: &str,
    bad_ordering: Ordering,
    attribution: &[Provenance],
    findings: &mut Vec<Finding>,
) -> Option<ScalarFieldEdit>
where
    T: DenyTomlScalar,
{
    if current
        .and_then(T::parse_item)
        .is_some_and(|value| value.compare_for_order(want) != Some(bad_ordering))
    {
        return None;
    }
    findings.push(Finding::Mismatch {
        key: display_key.to_owned(),
        selector: None,
        current: current.map(render_item),
        expected: T::render_value(want),
        message: message.to_owned(),
        severity: Severity::Error,
        attribution: attribution.to_vec(),
    });
    Some(ScalarFieldEdit::Write(T::write_item(want)))
}
