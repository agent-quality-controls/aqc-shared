//! `[package].<field>` / `[workspace.package].<field>` assertions.

#![expect(
    clippy::type_complexity,
    reason = "Collected assertions are plainly Vec<(Provenance, A)> and per-key maps of them; the shapes are declared openly at every signature instead of hidden behind wrapper types or aliases (taxonomy decision 2026-06-07)."
)]
use std::collections::BTreeSet;

use aqc_file_engine_core::{
    ConfigScalar, ConflictEntry, Msg, OnEmpty, OnEmptyClass, Provenance, Resolve,
    parse_version_tuple, resolve_scalar, union_string_lists, union_string_sets,
};

/// What must hold about a single `[package].<field>` (or
/// `[workspace.package].<field>`). One enum covers scalar and list-shaped
/// fields; the engine knows from the field key which shape applies.
///
/// Equality (and therefore merge agreement) compares the semantic value only;
/// the policy-authored [`Msg`] never participates.
#[derive(Debug, Clone)]
pub enum PackageFieldAssertion {
    /// The field equals this value (string, integer, or bool form).
    Equals(ConfigScalar, Msg),
    /// Version-ordered floor (rust-version, version, edition).
    AtLeastVersion(String, Msg),
    /// The field's value is one of these (check-only: the engine cannot pick).
    OneOf(BTreeSet<String>, Msg),
    /// The list field contains every element.
    ListContains(Vec<String>, Msg),
    /// The list field contains none of these elements.
    ListExcludes(BTreeSet<String>, Msg),
    /// The list field equals exactly this list.
    ListIsExactly(Vec<String>, Msg),
    /// The field uses workspace inheritance: `<field>.workspace = true`.
    InheritsWorkspace(Msg),
    /// The field is set, to anything (check-only).
    Present(Msg),
    /// The field is not set.
    Absent(Msg),
}

/// Semantic equality: messages excluded.
impl PartialEq for PackageFieldAssertion {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Equals(a, _), Self::Equals(b, _)) => a == b,
            (Self::AtLeastVersion(a, _), Self::AtLeastVersion(b, _)) => a == b,
            (Self::OneOf(a, _), Self::OneOf(b, _))
            | (Self::ListExcludes(a, _), Self::ListExcludes(b, _)) => a == b,
            (Self::ListContains(a, _), Self::ListContains(b, _))
            | (Self::ListIsExactly(a, _), Self::ListIsExactly(b, _)) => a == b,
            (Self::InheritsWorkspace(_), Self::InheritsWorkspace(_))
            | (Self::Present(_), Self::Present(_))
            | (Self::Absent(_), Self::Absent(_)) => true,
            _ => false,
        }
    }
}

impl Resolve for PackageFieldAssertion {
    fn resolve(
        key: &str,
        contributions: Vec<(Provenance, Self)>,
        conflicts: &mut Vec<ConflictEntry>,
    ) -> Option<Self> {
        // Two floors compose to the higher floor (max-wins), not a conflict:
        // `AtLeastVersion` is jointly satisfiable by definition.
        if contributions
            .iter()
            .all(|(_, a)| matches!(a, Self::AtLeastVersion(..)))
        {
            return Some(max_floor(contributions));
        }
        // Set-family variants union their elements; everything else must agree.
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

impl OnEmptyClass for PackageFieldAssertion {
    fn on_empty(&self) -> OnEmpty {
        match self {
            Self::Equals(..)
            | Self::AtLeastVersion(..)
            | Self::ListContains(..)
            | Self::ListExcludes(..)
            | Self::ListIsExactly(..)
            | Self::InheritsWorkspace(..)
            | Self::Absent(..) => OnEmpty::Writes,
            Self::OneOf(..) | Self::Present(..) => OnEmpty::ChecksOnly,
        }
    }
}

/// Resolve all-`AtLeastVersion` contributions to the highest floor: two
/// "at least X" requirements are jointly satisfied by the larger X. The
/// winning floor's message is kept.
fn max_floor(contributions: Vec<(Provenance, PackageFieldAssertion)>) -> PackageFieldAssertion {
    let mut best: Option<(String, Msg)> = None;
    for (_, assertion) in contributions {
        if let PackageFieldAssertion::AtLeastVersion(version, msg) = assertion {
            let take = best
                .as_ref()
                .is_none_or(|(b, _)| parse_version_tuple(&version) > parse_version_tuple(b));
            if take {
                best = Some((version, msg));
            }
        }
    }
    let (version, msg) = best.unwrap_or_else(|| (String::new(), String::new()));
    PackageFieldAssertion::AtLeastVersion(version, msg)
}

/// Union `ListContains` element lists via the core helper.
fn union_list_contains(
    contributions: Vec<(Provenance, PackageFieldAssertion)>,
) -> PackageFieldAssertion {
    let lists = contributions
        .into_iter()
        .filter_map(|(_, a)| {
            if let PackageFieldAssertion::ListContains(list, m) = a {
                Some((list, m))
            } else {
                None
            }
        })
        .collect();
    let (items, msg) = union_string_lists(lists);
    PackageFieldAssertion::ListContains(items, msg)
}

/// Union `ListExcludes` element sets via the core helper.
fn union_list_excludes(
    contributions: Vec<(Provenance, PackageFieldAssertion)>,
) -> PackageFieldAssertion {
    let sets = contributions
        .into_iter()
        .filter_map(|(_, a)| {
            if let PackageFieldAssertion::ListExcludes(set, m) = a {
                Some((set, m))
            } else {
                None
            }
        })
        .collect();
    let (items, msg) = union_string_sets(sets);
    PackageFieldAssertion::ListExcludes(items, msg)
}
