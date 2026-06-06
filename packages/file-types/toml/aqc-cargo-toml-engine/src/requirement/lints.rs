//! `[lints.<tool>]` / `[workspace.lints.<tool>]` tables and the
//! `[lints] workspace = <bool>` member opt-in.

use std::collections::BTreeMap;

use aqc_file_engine_core::{
    ConflictEntry, FromEmpty, FromEmptyClass, Msg, Provenance, Resolve, resolve_scalar,
};

use super::macros::impl_set_resolve;

/// What must hold about a `[lints.<tool>]` table.
///
/// Map values are `(level, priority, message)` tuples.
///   * `level` is the lint level (`"deny"`, `"warn"`, `"allow"`, `"forbid"`).
///   * `priority` is `Some(i64)` for inline-table form (used for group lints
///     like `clippy::all` with `priority = -1`); `None` for bare-string form.
///   * `message` is the policy-authored explanation surfaced in the
///     `Finding::Mismatch.message` field when this entry disagrees with disk.
///
/// Format choice is policy intent: bare-string for individual lints,
/// inline-table for group lints that need to be applied before per-lint
/// settings. The engine reads both forms but writes whichever form the
/// policy's `priority` slot implies.
///
/// `LintEntries` is the `(level, priority, message)` map both `Contains` and
/// `IsExactly` carry.
pub type LintEntries = BTreeMap<String, LintEntry>;

/// One lint entry: `(level, priority, message)`.
pub type LintEntry = (String, Option<i64>, String);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LintLevelsAssertion {
    /// Each (lint name, (level, priority, message)) mapping must be present on disk.
    Contains(LintEntries),
    /// None of these lint names may be set on disk. Value is the message.
    Excludes(BTreeMap<String, String>),
    /// The table must equal exactly these (name -> (level, priority, message)) entries.
    IsExactly(LintEntries),
}

impl_set_resolve!(LintLevelsAssertion, LintEntries, |entry: &LintEntry| (
    entry.0.clone(),
    entry.1
));

impl FromEmptyClass for LintLevelsAssertion {
    fn on_empty(&self) -> FromEmpty {
        // Contains/IsExactly write the entries; Excludes is vacuously satisfied.
        FromEmpty::Writes
    }
}

/// Whether `[lints]` inherits the workspace lint tables (`workspace = <bool>`).
///
/// The member opt-in that makes `[workspace.lints.*]` actually apply to a
/// package. Cargo rejects a manifest combining `workspace = true` with inline
/// `[lints.<tool>]` tables; the engine reports that combination as an Error
/// instead of writing it.
///
/// Equality (and therefore merge agreement) ignores the policy message.
#[derive(Debug, Clone)]
pub enum LintsInheritAssertion {
    /// `[lints] workspace = <bool>`.
    Equals(bool, Msg),
    /// The `workspace` key is set, to anything (check-only).
    Present(Msg),
    /// The `workspace` key is not set.
    Absent(Msg),
}

/// Semantic equality: messages excluded.
impl PartialEq for LintsInheritAssertion {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Equals(a, _), Self::Equals(b, _)) => a == b,
            (Self::Present(_), Self::Present(_)) | (Self::Absent(_), Self::Absent(_)) => true,
            _ => false,
        }
    }
}

impl Resolve for LintsInheritAssertion {
    fn resolve(
        key: &str,
        contributions: Vec<(Provenance, Self)>,
        conflicts: &mut Vec<ConflictEntry>,
    ) -> Option<Self> {
        resolve_scalar(key, contributions, |a| format!("{a:?}"), conflicts)
    }
}

impl FromEmptyClass for LintsInheritAssertion {
    fn on_empty(&self) -> FromEmpty {
        match self {
            Self::Equals(..) | Self::Absent(..) => FromEmpty::Writes,
            Self::Present(..) => FromEmpty::ChecksOnly,
        }
    }
}
