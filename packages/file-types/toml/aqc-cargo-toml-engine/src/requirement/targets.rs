//! Target-table assertions: `[lib]` fields and the named `[[bin]]` /
//! `[[example]]` / `[[test]]` / `[[bench]]` entries.

use std::collections::{BTreeMap, BTreeSet};

use aqc_file_engine_core::merge::Contributions;
use aqc_file_engine_core::{
    ConfigScalar, ConflictEntry, FromEmpty, FromEmptyClass, Msg, Provenance, Resolve, merge_map,
    resolve_scalar, union_string_lists,
};

/// What must hold about a single target-table field (`path`, `harness`,
/// `doctest`, `crate-type`, `required-features`, ...).
///
/// Equality (and therefore merge agreement) ignores the policy message.
#[derive(Debug, Clone)]
pub enum TargetFieldAssertion {
    /// The field equals this value.
    Equals(ConfigScalar, Msg),
    /// The field's value is one of these (check-only).
    OneOf(BTreeSet<String>, Msg),
    /// The list field contains every element (`crate-type` contains `"rlib"`).
    ListContains(Vec<String>, Msg),
    /// The list field equals exactly this list.
    ListIsExactly(Vec<String>, Msg),
    /// The field is set, to anything (check-only).
    Present(Msg),
    /// The field is not set.
    Absent(Msg),
}

/// Semantic equality: messages excluded.
impl PartialEq for TargetFieldAssertion {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Equals(a, _), Self::Equals(b, _)) => a == b,
            (Self::OneOf(a, _), Self::OneOf(b, _)) => a == b,
            (Self::ListContains(a, _), Self::ListContains(b, _))
            | (Self::ListIsExactly(a, _), Self::ListIsExactly(b, _)) => a == b,
            (Self::Present(_), Self::Present(_)) | (Self::Absent(_), Self::Absent(_)) => true,
            _ => false,
        }
    }
}

impl Resolve for TargetFieldAssertion {
    fn resolve(
        key: &str,
        contributions: Vec<(Provenance, Self)>,
        conflicts: &mut Vec<ConflictEntry>,
    ) -> Option<Self> {
        if contributions
            .iter()
            .all(|(_, a)| matches!(a, Self::ListContains(..)))
        {
            return Some(union_list_contains(contributions));
        }
        resolve_scalar(key, contributions, |a| format!("{a:?}"), conflicts)
    }
}

impl FromEmptyClass for TargetFieldAssertion {
    fn on_empty(&self) -> FromEmpty {
        match self {
            Self::Equals(..)
            | Self::ListContains(..)
            | Self::ListIsExactly(..)
            | Self::Absent(..) => FromEmpty::Writes,
            Self::OneOf(..) | Self::Present(..) => FromEmpty::ChecksOnly,
        }
    }
}

/// What must hold about one named array-of-tables target (`[[bin]]` etc.).
///
/// The map key supplies the required `name`, so `Present`/`Fields` are
/// writable; unasserted fields fall to cargo's auto-discovery defaults.
#[derive(Debug, Clone)]
pub enum TargetTableAssertion {
    /// A target with this name exists.
    Present(Msg),
    /// No target with this name exists.
    Absent(Msg),
    /// A target with this name exists and these fields hold.
    Fields(BTreeMap<String, TargetFieldAssertion>),
}

/// Semantic equality: messages excluded.
impl PartialEq for TargetTableAssertion {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Present(_), Self::Present(_)) | (Self::Absent(_), Self::Absent(_)) => true,
            (Self::Fields(a), Self::Fields(b)) => a == b,
            _ => false,
        }
    }
}

impl Resolve for TargetTableAssertion {
    fn resolve(
        key: &str,
        contributions: Vec<(Provenance, Self)>,
        conflicts: &mut Vec<ConflictEntry>,
    ) -> Option<Self> {
        if contributions
            .iter()
            .all(|(_, a)| matches!(a, Self::Fields(_)))
        {
            let maps = contributions
                .into_iter()
                .filter_map(|(p, a)| match a {
                    Self::Fields(m) => Some((p, m)),
                    Self::Present(_) | Self::Absent(_) => None,
                })
                .collect();
            return Some(Self::Fields(merge_map(
                key,
                maps,
                |a| format!("{a:?}"),
                conflicts,
            )));
        }
        resolve_scalar(key, contributions, |a| format!("{a:?}"), conflicts)
    }
}

impl FromEmptyClass for TargetTableAssertion {
    fn on_empty(&self) -> FromEmpty {
        match self {
            Self::Present(_) | Self::Absent(_) => FromEmpty::Writes,
            // Writable when every asserted field is writable.
            Self::Fields(map) => {
                if map.values().any(|a| a.on_empty() == FromEmpty::ChecksOnly) {
                    FromEmpty::ChecksOnly
                } else {
                    FromEmpty::Writes
                }
            }
        }
    }
}

/// Union `ListContains` element lists via the core helper.
fn union_list_contains(contributions: Contributions<TargetFieldAssertion>) -> TargetFieldAssertion {
    let lists = contributions
        .into_iter()
        .filter_map(|(_, a)| {
            if let TargetFieldAssertion::ListContains(list, m) = a {
                Some((list, m))
            } else {
                None
            }
        })
        .collect();
    let (items, msg) = union_string_lists(lists);
    TargetFieldAssertion::ListContains(items, msg)
}
