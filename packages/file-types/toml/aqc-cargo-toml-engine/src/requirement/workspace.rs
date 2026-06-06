//! `[workspace].<key>` assertions (resolver, members, exclude, default-members).

use std::collections::BTreeSet;

use aqc_file_engine_core::merge::Contributions;
use aqc_file_engine_core::{
    ConfigScalar, ConflictEntry, FromEmpty, FromEmptyClass, Msg, Provenance, Resolve,
    resolve_scalar, union_string_lists, union_string_sets,
};

/// What must hold about a direct `[workspace]` key.
///
/// Equality (and therefore merge agreement) compares the semantic value only;
/// the policy-authored [`Msg`] never participates.
#[derive(Debug, Clone)]
pub enum WorkspaceFieldAssertion {
    /// The key equals this value (`resolver = "3"`).
    Equals(ConfigScalar, Msg),
    /// The key's value is one of these (check-only).
    OneOf(BTreeSet<String>, Msg),
    /// The list key contains every element (`members` contains `"packages/*"`).
    ListContains(Vec<String>, Msg),
    /// The list key contains none of these elements.
    ListExcludes(BTreeSet<String>, Msg),
    /// The list key equals exactly this list.
    ListIsExactly(Vec<String>, Msg),
    /// The key is set, to anything (check-only).
    Present(Msg),
    /// The key is not set.
    Absent(Msg),
}

/// Semantic equality: messages excluded.
impl PartialEq for WorkspaceFieldAssertion {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Equals(a, _), Self::Equals(b, _)) => a == b,
            (Self::OneOf(a, _), Self::OneOf(b, _))
            | (Self::ListExcludes(a, _), Self::ListExcludes(b, _)) => a == b,
            (Self::ListContains(a, _), Self::ListContains(b, _))
            | (Self::ListIsExactly(a, _), Self::ListIsExactly(b, _)) => a == b,
            (Self::Present(_), Self::Present(_)) | (Self::Absent(_), Self::Absent(_)) => true,
            _ => false,
        }
    }
}

impl Resolve for WorkspaceFieldAssertion {
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
        if contributions
            .iter()
            .all(|(_, a)| matches!(a, Self::ListExcludes(..)))
        {
            return Some(union_list_excludes(contributions));
        }
        resolve_scalar(key, contributions, |a| format!("{a:?}"), conflicts)
    }
}

impl FromEmptyClass for WorkspaceFieldAssertion {
    fn on_empty(&self) -> FromEmpty {
        match self {
            Self::Equals(..)
            | Self::ListContains(..)
            | Self::ListExcludes(..)
            | Self::ListIsExactly(..)
            | Self::Absent(..) => FromEmpty::Writes,
            Self::OneOf(..) | Self::Present(..) => FromEmpty::ChecksOnly,
        }
    }
}

/// Union `ListContains` element lists via the core helper.
fn union_list_contains(
    contributions: Contributions<WorkspaceFieldAssertion>,
) -> WorkspaceFieldAssertion {
    let lists = contributions
        .into_iter()
        .filter_map(|(_, a)| {
            if let WorkspaceFieldAssertion::ListContains(list, m) = a {
                Some((list, m))
            } else {
                None
            }
        })
        .collect();
    let (items, msg) = union_string_lists(lists);
    WorkspaceFieldAssertion::ListContains(items, msg)
}

/// Union `ListExcludes` element sets via the core helper.
fn union_list_excludes(
    contributions: Contributions<WorkspaceFieldAssertion>,
) -> WorkspaceFieldAssertion {
    let sets = contributions
        .into_iter()
        .filter_map(|(_, a)| {
            if let WorkspaceFieldAssertion::ListExcludes(set, m) = a {
                Some((set, m))
            } else {
                None
            }
        })
        .collect();
    let (items, msg) = union_string_sets(sets);
    WorkspaceFieldAssertion::ListExcludes(items, msg)
}
