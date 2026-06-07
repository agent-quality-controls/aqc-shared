//! `[lints.<tool>]` / `[workspace.lints.<tool>]` tables and the
//! `[lints] workspace = <bool>` member opt-in.

#![expect(
    clippy::type_complexity,
    reason = "Collected assertions are plainly Vec<(Provenance, A)> and per-key maps of them; the shapes are declared openly at every signature instead of hidden behind wrapper types or aliases (taxonomy decision 2026-06-07)."
)]
use std::collections::BTreeMap;

use aqc_file_engine_core::{
    ConflictEntry, Msg, OnEmpty, OnEmptyClass, Provenance, Resolve, resolve_scalar,
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

impl OnEmptyClass for LintLevelsAssertion {
    fn on_empty(&self) -> OnEmpty {
        // Contains/IsExactly write the entries; Excludes is vacuously satisfied.
        OnEmpty::Writes
    }
}

/// `[lints]` is ONE either/or decision (cargo's own rule).
///
/// A manifest either inherits the workspace tables (`workspace = <bool>`) or
/// carries inline `[lints.<tool>]` tables -- never both. Modeling it as one
/// key makes the inline-vs-inherit combination an ordinary merge conflict
/// (`ConflictingRequirements` naming both policies) instead of an unwritable
/// state the engine would have to refuse ad hoc.
#[derive(Debug, Clone)]
pub enum PackageLintsAssertion {
    /// `[lints] workspace = <bool>` (the member opt-in).
    Inherit(bool, Msg),
    /// Inline `[lints.<tool>]` tables, keyed by tool.
    Inline(BTreeMap<String, LintLevelsAssertion>),
}

/// Semantic equality: the `Inherit` message is excluded.
impl PartialEq for PackageLintsAssertion {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Inherit(a, _), Self::Inherit(b, _)) => a == b,
            (Self::Inline(a), Self::Inline(b)) => a == b,
            _ => false,
        }
    }
}

impl Resolve for PackageLintsAssertion {
    fn resolve(
        key: &str,
        contributions: Vec<(Provenance, Self)>,
        conflicts: &mut Vec<ConflictEntry>,
    ) -> Option<Self> {
        let all_inherit = contributions
            .iter()
            .all(|(_, a)| matches!(a, Self::Inherit(..)));
        let all_inline = contributions
            .iter()
            .all(|(_, a)| matches!(a, Self::Inline(..)));
        if all_inherit {
            return resolve_scalar(key, contributions, |a| format!("{a:?}"), conflicts);
        }
        if all_inline {
            return Some(Self::Inline(resolve_inline(key, contributions, conflicts)));
        }
        // Mixed inherit + inline: cargo rejects the combination; the policy
        // set disagrees on the one [lints] decision.
        conflicts.push(ConflictEntry {
            key: key.to_owned(),
            contributors: contributions
                .into_iter()
                .map(|(p, a)| {
                    let rendered = match a {
                        Self::Inherit(b, _) => format!("inherit (workspace = {b})"),
                        Self::Inline(tools) => {
                            let names: Vec<&str> = tools.keys().map(String::as_str).collect();
                            format!("inline [lints.<tool>] tables ({})", names.join(", "))
                        }
                    };
                    (p, rendered)
                })
                .collect(),
            reason: "scalar-disagree".to_owned(),
        });
        None
    }
}

impl OnEmptyClass for PackageLintsAssertion {
    fn on_empty(&self) -> OnEmpty {
        // Both forms have one correct value; init writes it.
        OnEmpty::Writes
    }
}

/// Union all-`Inline` contributions per tool through the lint-table merge
/// (message-insensitive); a per-tool disagreement drops that tool's table.
fn resolve_inline(
    key: &str,
    contributions: Vec<(Provenance, PackageLintsAssertion)>,
    conflicts: &mut Vec<ConflictEntry>,
) -> BTreeMap<String, LintLevelsAssertion> {
    let mut by_tool: BTreeMap<String, Vec<(Provenance, LintLevelsAssertion)>> = BTreeMap::new();
    for (prov, assertion) in contributions {
        let PackageLintsAssertion::Inline(tools) = assertion else {
            continue;
        };
        for (tool, table) in tools {
            by_tool.entry(tool).or_default().push((prov.clone(), table));
        }
    }
    let mut resolved = BTreeMap::new();
    for (tool, pairs) in by_tool {
        let tool_key = format!("{key}.{tool}");
        if let Some(table) = LintLevelsAssertion::resolve(&tool_key, pairs, conflicts) {
            let _ = resolved.insert(tool, table);
        }
    }
    resolved
}
