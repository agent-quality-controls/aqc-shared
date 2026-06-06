//! The shared from-empty contract harness.
//!
//! Binds an assertion's declared [`FromEmpty`] class to the engine's actual
//! reconcile behavior. Every engine runs these two laws over every variant of
//! every assertion enum in its test catalogue; a wrong declaration or a wrong
//! apply fails the law, so the declaration, the behavior, and the test hold
//! each other.
//!
//! The checks return `Result<(), ContractViolation>` (never panic) so engine tests can
//! `assert!` on them with a message under the workspace's no-panic lints.

use crate::finding::Finding;
use crate::types::{EngineOutput, FromEmpty, Severity};

/// A from-empty law violation, carrying the human-readable description the
/// failing test surfaces.
#[derive(Debug)]
#[expect(
    clippy::module_name_repetitions,
    reason = "`ContractViolation` is the harness's error; the name reads at call sites and pairs with the contract module."
)]
pub struct ContractViolation {
    /// The law that was broken, described for the failing test's output.
    detail: String,
}

impl core::fmt::Display for ContractViolation {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(&self.detail)
    }
}

impl core::error::Error for ContractViolation {}

impl ContractViolation {
    /// Wrap a law-violation description.
    const fn new(detail: String) -> Self {
        Self { detail }
    }
}

/// Run the law for `class` against `reconcile` with a single-assertion
/// `requirement`, starting from a missing file.
///
/// `Writes` law: the first pass must carry no hard-failure finding
/// (parse/schema/internal/conflict), and reconciling its own output again
/// must be completely clean and byte-identical (write once, then settled).
///
/// `ChecksOnly` law: the first pass must write nothing (no bytes emitted for
/// a missing file) and report at least one Error finding; a second pass over
/// its own output must still report an Error (it never converges on its own).
///
/// # Errors
///
/// Returns a human-readable description of the first law violation found.
pub fn check_from_empty<Req>(
    reconcile: impl Fn(Option<&[u8]>, &Req) -> EngineOutput,
    requirement: &Req,
    class: FromEmpty,
) -> Result<(), ContractViolation> {
    let first = reconcile(None, requirement);
    match class {
        FromEmpty::Writes => check_writes_law(&reconcile, requirement, &first),
        FromEmpty::ChecksOnly => check_checks_only_law(&reconcile, requirement, &first),
    }
}

/// The `Writes` law body. See [`check_from_empty`].
fn check_writes_law<Req>(
    reconcile: &impl Fn(Option<&[u8]>, &Req) -> EngineOutput,
    requirement: &Req,
    first: &EngineOutput,
) -> Result<(), ContractViolation> {
    if let Some(hard) = first.findings.iter().find(|f| is_hard(f)) {
        return Err(ContractViolation::new(format!(
            "Writes law: first pass from empty produced a hard-failure finding: {hard:?}"
        )));
    }
    let second = reconcile(Some(&first.expected_bytes), requirement);
    if !second.findings.is_empty() {
        return Err(ContractViolation::new(format!(
            "Writes law: reconciling the engine's own output is not clean: {:?}",
            second.findings
        )));
    }
    if second.expected_bytes != first.expected_bytes {
        return Err(ContractViolation::new(format!(
            "Writes law: not idempotent; second pass changed the bytes.\nfirst:\n{}\nsecond:\n{}",
            String::from_utf8_lossy(&first.expected_bytes),
            String::from_utf8_lossy(&second.expected_bytes),
        )));
    }
    Ok(())
}

/// The `ChecksOnly` law body. See [`check_from_empty`].
fn check_checks_only_law<Req>(
    reconcile: &impl Fn(Option<&[u8]>, &Req) -> EngineOutput,
    requirement: &Req,
    first: &EngineOutput,
) -> Result<(), ContractViolation> {
    if !first.expected_bytes.is_empty() {
        return Err(ContractViolation::new(format!(
            "ChecksOnly law: wrote content from an empty file:\n{}",
            String::from_utf8_lossy(&first.expected_bytes)
        )));
    }
    if !has_error(&first.findings) {
        return Err(ContractViolation::new(format!(
            "ChecksOnly law: no Error finding from an empty file; findings: {:?}",
            first.findings
        )));
    }
    let second = reconcile(Some(&first.expected_bytes), requirement);
    if !has_error(&second.findings) {
        return Err(ContractViolation::new(
            "ChecksOnly law: converged on its own output; a check-only assertion never resolves itself"
                .to_owned(),
        ));
    }
    Ok(())
}

/// True when any finding carries `Severity::Error` (or is implicitly Error).
fn has_error(findings: &[Finding]) -> bool {
    findings.iter().any(|f| severity_of(f) == Severity::Error)
}

/// A finding `init` must never write through: the file (or the requirement
/// set) is broken, not merely drifted.
const fn is_hard(finding: &Finding) -> bool {
    matches!(
        finding,
        Finding::ParseError { .. }
            | Finding::SchemaError { .. }
            | Finding::InternalError { .. }
            | Finding::PolicyConflict { .. }
    )
}

/// The effective severity of a finding (variants without a severity field are
/// always Error by contract).
const fn severity_of(finding: &Finding) -> Severity {
    match finding {
        Finding::Mismatch { severity, .. }
        | Finding::SchemaError { severity, .. }
        | Finding::ParseError { severity, .. } => *severity,
        Finding::UnwritableRequiredKey { .. }
        | Finding::PolicyConflict { .. }
        | Finding::InternalError { .. } => Severity::Error,
    }
}
