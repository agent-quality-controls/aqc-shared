//! Table-level existence assertions per manifest section.

use aqc_file_engine_core::{
    ConflictEntry, OnEmpty, Provenance, Resolve, ResolvedRequirement, resolve_scalar,
};

/// The manifest sections whose *existence* is controllable. Closed set (D7):
/// presence control only for sections whose content is either covered by a
/// richer target or deliberately uncontrolled.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ManifestSection {
    Workspace,
    WorkspaceLints,
    Package,
    Lib,
    Badges,
    Replace,
    Patch,
    CargoFeatures,
    Hints,
}

impl ManifestSection {
    /// The in-file table path this section controls (used on findings).
    #[must_use]
    pub const fn table_path(self) -> &'static str {
        match self {
            Self::Workspace => "[workspace]",
            Self::WorkspaceLints => "[workspace.lints]",
            Self::Package => "[package]",
            Self::Lib => "[lib]",
            Self::Badges => "[badges]",
            Self::Replace => "[replace]",
            Self::Patch => "[patch]",
            Self::CargoFeatures => "[cargo-features]",
            Self::Hints => "[hints]",
        }
    }

    /// The top-level TOML key of this section.
    #[must_use]
    pub const fn key(self) -> &'static str {
        match self {
            Self::Workspace => "workspace",
            Self::WorkspaceLints => "lints",
            Self::Package => "package",
            Self::Lib => "lib",
            Self::Badges => "badges",
            Self::Replace => "replace",
            Self::Patch => "patch",
            Self::CargoFeatures => "cargo-features",
            Self::Hints => "hints",
        }
    }
}

/// Whether the section's table must exist or must not.
///
/// Equality (and therefore merge agreement) ignores the policy message.
#[derive(Debug, Clone)]
pub enum SectionPresenceAssertion {
    /// The table exists (content unconstrained here).
    Present(String),
    /// The table does not exist.
    Absent(String),
}

/// Semantic equality: messages excluded.
impl PartialEq for SectionPresenceAssertion {
    fn eq(&self, other: &Self) -> bool {
        matches!(
            (self, other),
            (Self::Present(_), Self::Present(_)) | (Self::Absent(_), Self::Absent(_))
        )
    }
}

impl Resolve for SectionPresenceAssertion {
    type Merged = Self;

    fn resolve(
        key: &str,
        items: Vec<(Provenance, Self)>,
        conflicts: &mut Vec<ConflictEntry>,
    ) -> Option<ResolvedRequirement<Self::Merged, Self>> {
        resolve_scalar(key, items, render_section_presence_assertion, conflicts)
    }
}

/// Renders section-presence assertions for conflict contributor output.
fn render_section_presence_assertion(assertion: &SectionPresenceAssertion) -> String {
    match assertion {
        SectionPresenceAssertion::Present(_) => "present".to_owned(),
        SectionPresenceAssertion::Absent(_) => "absent".to_owned(),
    }
}

impl SectionPresenceAssertion {
    /// The class depends on the section, so it is answered by the pair
    /// (an orphan-rule-safe inherent method instead of [`OnEmptyClass`]):
    /// `Present([workspace])` writes an empty table; `Present([package])` is
    /// check-only because `[package]` needs a `name` the engine cannot
    /// invent. `Absent` always writes (nothing to do on an empty file;
    /// delete when present).
    #[must_use]
    pub const fn on_empty_in(&self, section: ManifestSection) -> OnEmpty {
        match (section, self) {
            (ManifestSection::Package, Self::Present(_)) => OnEmpty::ChecksOnly,
            (_, Self::Absent(_) | Self::Present(_)) => OnEmpty::Writes,
        }
    }
}
