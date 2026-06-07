//! Declarative requirement and assertion types accepted by `ClippyTomlEngine`.

#![expect(
    clippy::type_complexity,
    reason = "Collected assertions are plainly Vec<(Provenance, A)> and per-key maps of them; the shapes are declared openly at every signature instead of hidden behind wrapper types or aliases (taxonomy decision 2026-06-07)."
)]
#![expect(
    clippy::disallowed_types,
    reason = "`Any` is used only in the `EngineRequirement::as_any` impl; the broker uses it to downcast `Box<dyn EngineRequirement>` back to this concrete `Req` type at dispatch time."
)]

use core::any::Any;
use std::collections::{BTreeMap, BTreeSet};

use aqc_file_engine_core::{
    ConflictEntry, EngineRequirement, Provenance, Resolve, resolve_field, resolve_optional,
    resolve_scalar, union_field, union_optional,
};

/// Declarative requirement for the `clippy.toml` engine.
///
/// One field per addressable section. Each field's value is a
/// `MergedAssertion<...>` (or map thereof) carrying the per-policy
/// contributions.
#[derive(Debug, Clone, Default)]
pub struct ClippyTomlRequirement {
    pub msrv: Option<Vec<(Provenance, MsrvAssertion)>>,
    pub thresholds: Option<Vec<(Provenance, ThresholdsAssertion)>>,
    pub disallowed_methods: Option<Vec<(Provenance, BansAssertion)>>,
    pub disallowed_types: Option<Vec<(Provenance, BansAssertion)>>,
    pub disallowed_macros: Option<Vec<(Provenance, BansAssertion)>>,
    pub bools: BTreeMap<String, Vec<(Provenance, BoolAssertion)>>,
    pub enums: BTreeMap<String, Vec<(Provenance, StringAssertion)>>,
}

impl ClippyTomlRequirement {
    /// Merge a slice of requirements (all routed to this engine for one file)
    /// into one resolved requirement plus any per-key conflicts.
    ///
    /// Phase 1 of reconciliation: pure, disk-independent. Per field, union the
    /// contributions, then resolve each key (identical → collapse, set/map →
    /// union keys, disagreement → [`ConflictEntry`]). The engine turns each
    /// entry into a `Finding::ConflictingRequirements`.
    #[must_use]
    pub fn merge(reqs: &[&Self]) -> (Self, Vec<ConflictEntry>) {
        let mut u = Self::default();
        for r in reqs {
            u.msrv = union_optional(u.msrv.take(), r.msrv.clone());
            u.thresholds = union_optional(u.thresholds.take(), r.thresholds.clone());
            u.disallowed_methods =
                union_optional(u.disallowed_methods.take(), r.disallowed_methods.clone());
            u.disallowed_types =
                union_optional(u.disallowed_types.take(), r.disallowed_types.clone());
            u.disallowed_macros =
                union_optional(u.disallowed_macros.take(), r.disallowed_macros.clone());
            union_field(&mut u.bools, r.bools.clone());
            union_field(&mut u.enums, r.enums.clone());
        }
        let mut conflicts = Vec::new();
        let out = Self {
            msrv: resolve_optional("msrv", u.msrv, &mut conflicts),
            thresholds: resolve_optional("[thresholds]", u.thresholds, &mut conflicts),
            disallowed_methods: resolve_optional(
                "disallowed-methods",
                u.disallowed_methods,
                &mut conflicts,
            ),
            disallowed_types: resolve_optional(
                "disallowed-types",
                u.disallowed_types,
                &mut conflicts,
            ),
            disallowed_macros: resolve_optional(
                "disallowed-macros",
                u.disallowed_macros,
                &mut conflicts,
            ),
            bools: resolve_field(u.bools, Clone::clone, &mut conflicts),
            enums: resolve_field(u.enums, Clone::clone, &mut conflicts),
        };
        (out, conflicts)
    }
}

/// What must hold about `msrv`. Each value-carrying variant carries a
/// `(value, message)` pair; `Present`/`Absent` carry the message.
#[derive(Debug, Clone)]
pub enum MsrvAssertion {
    /// `(version, message)`.
    Equals(String, String),
    /// `(version, message)`.
    AtLeast(String, String),
    /// `(allowed versions, message)`.
    OneOf(BTreeSet<String>, String),
    /// `(message)` -- msrv must be set.
    Present(String),
    /// `(message)` -- msrv must not be set.
    Absent(String),
}

/// Equality ignores the policy-authored message: two policies asserting the
/// same semantic value disagree only when the value differs.
impl PartialEq for MsrvAssertion {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Equals(a, _), Self::Equals(b, _))
            | (Self::AtLeast(a, _), Self::AtLeast(b, _)) => a == b,
            (Self::OneOf(a, _), Self::OneOf(b, _)) => a == b,
            (Self::Present(_), Self::Present(_)) | (Self::Absent(_), Self::Absent(_)) => true,
            (
                Self::Equals(..)
                | Self::AtLeast(..)
                | Self::OneOf(..)
                | Self::Present(_)
                | Self::Absent(_),
                _,
            ) => false,
        }
    }
}

/// What must hold about clippy's numeric threshold keys
/// (e.g. `cognitive-complexity-threshold`). Map values are
/// `(threshold, message)` pairs; set-shaped variants map name -> message.
#[derive(Debug, Clone)]
pub enum ThresholdsAssertion {
    Equals(BTreeMap<String, (u64, String)>),
    AtMost(BTreeMap<String, (u64, String)>),
    AtLeast(BTreeMap<String, (u64, String)>),
    /// name -> message (key must be present).
    Present(BTreeMap<String, String>),
    /// name -> message (key must be absent).
    Absent(BTreeMap<String, String>),
}

/// Equality ignores the policy-authored message: the value-carrying variants
/// compare only the numeric threshold per key; the set-shaped variants compare
/// only the key sets. Different variants never agree.
impl PartialEq for ThresholdsAssertion {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Equals(a), Self::Equals(b))
            | (Self::AtMost(a), Self::AtMost(b))
            | (Self::AtLeast(a), Self::AtLeast(b)) => values_eq(a, b),
            (Self::Present(a), Self::Present(b)) | (Self::Absent(a), Self::Absent(b)) => {
                keys_eq(a, b)
            }
            (
                Self::Equals(_)
                | Self::AtMost(_)
                | Self::AtLeast(_)
                | Self::Present(_)
                | Self::Absent(_),
                _,
            ) => false,
        }
    }
}

/// Compare two `(value, message)` maps by key and numeric value only.
fn values_eq(a: &BTreeMap<String, (u64, String)>, b: &BTreeMap<String, (u64, String)>) -> bool {
    a.len() == b.len()
        && a.iter()
            .all(|(k, (v, _))| b.get(k).is_some_and(|(w, _)| v == w))
}

/// Compare two `name -> message` maps by key set only.
fn keys_eq(a: &BTreeMap<String, String>, b: &BTreeMap<String, String>) -> bool {
    a.len() == b.len() && a.keys().all(|k| b.contains_key(k))
}

/// What must hold about a ban table (`disallowed-methods`, `disallowed-types`,
/// `disallowed-macros`). Same shape for all three; reused across targets.
#[derive(Debug, Clone)]
pub enum BansAssertion {
    Contains(Vec<BanEntry>),
    /// path -> message (must not be banned).
    Excludes(BTreeMap<String, String>),
    IsExactly(Vec<BanEntry>),
}

/// Equality ignores the policy-authored message: ban lists compare by path set
/// only; the exclusion map compares by path set only. Different variants never
/// agree.
impl PartialEq for BansAssertion {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Contains(a), Self::Contains(b)) | (Self::IsExactly(a), Self::IsExactly(b)) => {
                paths_eq(a, b)
            }
            (Self::Excludes(a), Self::Excludes(b)) => keys_eq(a, b),
            (Self::Contains(_) | Self::Excludes(_) | Self::IsExactly(_), _) => false,
        }
    }
}

/// Compare two ban-entry lists by their path sets only.
fn paths_eq(a: &[BanEntry], b: &[BanEntry]) -> bool {
    let pa: BTreeSet<&str> = a.iter().map(|e| e.path.as_str()).collect();
    let pb: BTreeSet<&str> = b.iter().map(|e| e.path.as_str()).collect();
    pa == pb
}

/// One entry in a clippy ban list. `message` is policy-mandatory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BanEntry {
    pub path: String,
    pub message: String,
}

/// What must hold about a boolean clippy setting.
#[derive(Debug, Clone)]
pub enum BoolAssertion {
    /// `(value, message)`.
    Equals(bool, String),
    /// `(message)`.
    Present(String),
    /// `(message)`.
    Absent(String),
}

/// Equality ignores the policy-authored message: two policies asserting the
/// same boolean disagree only when the value differs.
impl PartialEq for BoolAssertion {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Equals(a, _), Self::Equals(b, _)) => a == b,
            (Self::Present(_), Self::Present(_)) | (Self::Absent(_), Self::Absent(_)) => true,
            (Self::Equals(..) | Self::Present(_) | Self::Absent(_), _) => false,
        }
    }
}

/// What must hold about a string-valued (enum-style) clippy setting.
#[derive(Debug, Clone)]
pub enum StringAssertion {
    /// `(value, message)`.
    Equals(String, String),
    /// `(allowed values, message)`.
    OneOf(BTreeSet<String>, String),
    /// `(message)`.
    Present(String),
    /// `(message)`.
    Absent(String),
}

/// Equality ignores the policy-authored message: two policies asserting the
/// same value (or same allowed set) disagree only when that value differs.
impl PartialEq for StringAssertion {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Equals(a, _), Self::Equals(b, _)) => a == b,
            (Self::OneOf(a, _), Self::OneOf(b, _)) => a == b,
            (Self::Present(_), Self::Present(_)) | (Self::Absent(_), Self::Absent(_)) => true,
            (Self::Equals(..) | Self::OneOf(..) | Self::Present(_) | Self::Absent(_), _) => false,
        }
    }
}

impl EngineRequirement for ClippyTomlRequirement {
    fn engine_id(&self) -> &'static str {
        crate::ENGINE_ID
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

// ---------------------------------------------------------------------------
// `Resolve` impls: each assertion routes its variants to the core strategies.
// Identical contributions collapse; set/map variants union keys; scalar/exact
// disagreement becomes a `ConflictEntry`. The policy-authored message is never
// part of the disagreement (see the `PartialEq` impls above).
// ---------------------------------------------------------------------------

impl Resolve for MsrvAssertion {
    fn resolve(
        key: &str,
        contributions: Vec<(Provenance, Self)>,
        conflicts: &mut Vec<ConflictEntry>,
    ) -> Option<Self> {
        resolve_scalar(key, contributions, |a| format!("{a:?}"), conflicts)
    }
}

impl Resolve for BoolAssertion {
    fn resolve(
        key: &str,
        contributions: Vec<(Provenance, Self)>,
        conflicts: &mut Vec<ConflictEntry>,
    ) -> Option<Self> {
        resolve_scalar(key, contributions, |a| format!("{a:?}"), conflicts)
    }
}

impl Resolve for StringAssertion {
    fn resolve(
        key: &str,
        contributions: Vec<(Provenance, Self)>,
        conflicts: &mut Vec<ConflictEntry>,
    ) -> Option<Self> {
        resolve_scalar(key, contributions, |a| format!("{a:?}"), conflicts)
    }
}

impl Resolve for ThresholdsAssertion {
    fn resolve(
        key: &str,
        contributions: Vec<(Provenance, Self)>,
        conflicts: &mut Vec<ConflictEntry>,
    ) -> Option<Self> {
        if contributions
            .iter()
            .all(|(_, a)| matches!(a, Self::Equals(_)))
        {
            return Some(Self::Equals(union_threshold_values(
                key,
                contributions,
                conflicts,
            )));
        }
        if contributions
            .iter()
            .all(|(_, a)| matches!(a, Self::AtMost(_)))
        {
            return Some(Self::AtMost(union_threshold_values(
                key,
                contributions,
                conflicts,
            )));
        }
        if contributions
            .iter()
            .all(|(_, a)| matches!(a, Self::AtLeast(_)))
        {
            return Some(Self::AtLeast(union_threshold_values(
                key,
                contributions,
                conflicts,
            )));
        }
        if contributions
            .iter()
            .all(|(_, a)| matches!(a, Self::Present(_)))
        {
            return Some(Self::Present(union_threshold_sets(contributions)));
        }
        if contributions
            .iter()
            .all(|(_, a)| matches!(a, Self::Absent(_)))
        {
            return Some(Self::Absent(union_threshold_sets(contributions)));
        }
        resolve_scalar(key, contributions, |a| format!("{a:?}"), conflicts)
    }
}

impl Resolve for BansAssertion {
    fn resolve(
        key: &str,
        contributions: Vec<(Provenance, Self)>,
        conflicts: &mut Vec<ConflictEntry>,
    ) -> Option<Self> {
        if contributions
            .iter()
            .all(|(_, a)| matches!(a, Self::Contains(_)))
        {
            return Some(Self::Contains(union_ban_entries(contributions)));
        }
        if contributions
            .iter()
            .all(|(_, a)| matches!(a, Self::IsExactly(_)))
        {
            return Some(Self::IsExactly(union_ban_entries(contributions)));
        }
        if contributions
            .iter()
            .all(|(_, a)| matches!(a, Self::Excludes(_)))
        {
            return Some(Self::Excludes(union_ban_excludes(contributions)));
        }
        resolve_scalar(key, contributions, |a| format!("{a:?}"), conflicts)
    }
}

/// Union value-carrying threshold maps, keyed by threshold name.
///
/// Two policies setting the same key to a different numeric value →
/// one conflict keyed `{key}.{name}` (the policy-authored message is not part
/// of the disagreement). Same value with different messages collapse.
fn union_threshold_values(
    key: &str,
    contributions: Vec<(Provenance, ThresholdsAssertion)>,
    conflicts: &mut Vec<ConflictEntry>,
) -> BTreeMap<String, (u64, String)> {
    let mut by_name: BTreeMap<String, Vec<(Provenance, (u64, String))>> = BTreeMap::new();
    for (prov, a) in contributions {
        let map = match a {
            ThresholdsAssertion::Equals(m)
            | ThresholdsAssertion::AtMost(m)
            | ThresholdsAssertion::AtLeast(m) => m,
            ThresholdsAssertion::Present(_) | ThresholdsAssertion::Absent(_) => continue,
        };
        for (name, v) in map {
            by_name.entry(name).or_default().push((prov.clone(), v));
        }
    }
    let mut out: BTreeMap<String, (u64, String)> = BTreeMap::new();
    for (name, entries) in by_name {
        let mut iter = entries.into_iter();
        let Some((first_prov, first_val)) = iter.next() else {
            continue;
        };
        let mut contributors: Vec<(Provenance, String)> =
            vec![(first_prov, first_val.0.to_string())];
        let mut disagree = false;
        for (prov, v) in iter {
            if v.0 != first_val.0 {
                disagree = true;
            }
            contributors.push((prov, v.0.to_string()));
        }
        if disagree {
            conflicts.push(ConflictEntry {
                key: format!("{key}.{name}"),
                reason: "set-key-disagree".to_owned(),
                contributors,
            });
        } else {
            let _ = out.insert(name, first_val);
        }
    }
    out
}

/// Union set-shaped threshold maps, keyed by name; first message wins.
/// Pure union; the key set carries no value, so it never conflicts.
fn union_threshold_sets(
    contributions: Vec<(Provenance, ThresholdsAssertion)>,
) -> BTreeMap<String, String> {
    let mut out: BTreeMap<String, String> = BTreeMap::new();
    for (_, a) in contributions {
        let map = match a {
            ThresholdsAssertion::Present(m) | ThresholdsAssertion::Absent(m) => m,
            ThresholdsAssertion::Equals(_)
            | ThresholdsAssertion::AtMost(_)
            | ThresholdsAssertion::AtLeast(_) => continue,
        };
        for (name, msg) in map {
            let _ = out.entry(name).or_insert(msg);
        }
    }
    out
}

/// Union `Contains`/`IsExactly` ban lists across contributions, keyed by path.
///
/// Two policies banning the same path agree (the policy-authored message is not
/// part of the disagreement); the first entry's message wins. The path set is
/// the only value, so a shared path never conflicts.
fn union_ban_entries(contributions: Vec<(Provenance, BansAssertion)>) -> Vec<BanEntry> {
    let mut seen: BTreeSet<String> = BTreeSet::new();
    let mut out: Vec<BanEntry> = Vec::new();
    for (_, a) in contributions {
        let entries = match a {
            BansAssertion::Contains(v) | BansAssertion::IsExactly(v) => v,
            BansAssertion::Excludes(_) => continue,
        };
        for entry in entries {
            if seen.insert(entry.path.clone()) {
                out.push(entry);
            }
        }
    }
    out
}

/// Union `Excludes` ban maps across contributions, keyed by path; first message
/// wins. Pure union; never conflicts.
fn union_ban_excludes(contributions: Vec<(Provenance, BansAssertion)>) -> BTreeMap<String, String> {
    let mut out: BTreeMap<String, String> = BTreeMap::new();
    for (_, a) in contributions {
        let map = match a {
            BansAssertion::Excludes(m) => m,
            BansAssertion::Contains(_) | BansAssertion::IsExactly(_) => continue,
        };
        for (path, msg) in map {
            let _ = out.entry(path).or_insert(msg);
        }
    }
    out
}
