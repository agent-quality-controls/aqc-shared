//! Dependency-table assertions: `[dependencies]` / `[dev-dependencies]` /
//! `[build-dependencies]`, their `[target.'cfg(..)'.*]` variants,
//! `[workspace.dependencies]`, and `[patch.<registry>]` (same vocabulary).

use std::collections::BTreeMap;

use aqc_file_engine_core::{FromEmpty, FromEmptyClass, Msg};

use super::macros::{impl_keyed_entries_eq, impl_set_resolve};

/// Which dependency table kind. Names match cargo's own (`cargo metadata`
/// renders the pair as `dep_kinds: [{ kind, target }]`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DependencyKind {
    Normal,
    Dev,
    Build,
}

/// Which dependency table: kind plus the optional `cfg` platform
/// (`[target.'cfg(windows)'.dependencies]`). Field names match cargo's.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct DependencyScope {
    pub kind: DependencyKind,
    /// The `cfg` expression / target triple, when platform-scoped.
    pub target: Option<String>,
}

impl DependencyScope {
    /// The in-file table path this scope addresses (used on findings).
    #[must_use]
    pub fn table_path(&self) -> String {
        let kind = match self.kind {
            DependencyKind::Normal => "dependencies",
            DependencyKind::Dev => "dev-dependencies",
            DependencyKind::Build => "build-dependencies",
        };
        self.target
            .as_ref()
            .map_or_else(|| format!("[{kind}]"), |t| format!("[target.'{t}'.{kind}]"))
    }
}

/// Typed shape of one dependency entry.
///
/// A spec constrains **only the fields it sets** (partial matching, D4);
/// `IsExactly` stays the closed form at the set level. A spec is writable
/// only when it names a source (`version` | `path` | `git` | `workspace`);
/// cargo rejects a dependency entry with no source.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DependencySpec {
    pub version: Option<String>,
    pub features: Vec<String>,
    pub default_features: Option<bool>,
    pub optional: Option<bool>,
    /// `dep = { workspace = true }`: inherit from `[workspace.dependencies]`.
    pub workspace: Option<bool>,
    pub path: Option<String>,
    pub git: Option<String>,
    pub branch: Option<String>,
    pub tag: Option<String>,
    pub rev: Option<String>,
    pub registry: Option<String>,
    /// Rename: the registry package this entry actually points at.
    pub package: Option<String>,
}

impl DependencySpec {
    /// True when the spec names where the code comes from. Only then can the
    /// engine create the entry; without a source it can only check one.
    #[must_use]
    pub const fn has_source(&self) -> bool {
        self.version.is_some()
            || self.path.is_some()
            || self.git.is_some()
            || self.workspace.is_some()
    }
}

/// What must hold about one dependency table.
///
/// Equality (and therefore merge agreement) compares dependency names and
/// specs; the policy-authored messages never participate.
/// The entry map of a `Contains` / `IsExactly` assertion: dependency name to
/// its (partial) spec plus the policy message.
pub type DependencyEntries = BTreeMap<String, DependencyEntry>;

/// One dependency entry: the (partial) spec plus the policy message.
pub type DependencyEntry = (DependencySpec, Msg);

#[derive(Debug, Clone)]
pub enum DependencySetAssertion {
    /// These entries must be present, each matching its (partial) spec.
    Contains(DependencyEntries),
    /// None of these dependencies may be present (banned, with the why).
    Excludes(BTreeMap<String, Msg>),
    /// The table must contain exactly these entries.
    IsExactly(DependencyEntries),
}

impl_keyed_entries_eq!(DependencySetAssertion);
impl_set_resolve!(
    DependencySetAssertion,
    DependencyEntries,
    |entry: &DependencyEntry| entry.0.clone()
);

impl FromEmptyClass for DependencySetAssertion {
    fn on_empty(&self) -> FromEmpty {
        match self {
            // Writable only when every entry names a source; a sourceless
            // entry cannot be created (cargo rejects it), only checked.
            Self::Contains(map) | Self::IsExactly(map) => {
                if map.values().all(|(spec, _)| spec.has_source()) {
                    FromEmpty::Writes
                } else {
                    FromEmpty::ChecksOnly
                }
            }
            Self::Excludes(_) => FromEmpty::Writes,
        }
    }
}
