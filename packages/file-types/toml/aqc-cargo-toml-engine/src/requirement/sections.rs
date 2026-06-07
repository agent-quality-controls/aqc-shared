//! Table-level existence assertions per manifest section.

use aqc_file_engine_core::{ConflictEntry, Msg, OnEmpty, Provenance, Resolve, resolve_scalar};

/// The manifest sections whose *existence* is controllable. Closed set (D7):
/// presence control only for sections whose content is either covered by a
/// richer target or deliberately uncontrolled.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ManifestSection {
    Workspace,
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
    Present(Msg),
    /// The table does not exist.
    Absent(Msg),
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
    fn resolve(
        key: &str,
        contributions: Vec<(Provenance, Self)>,
        conflicts: &mut Vec<ConflictEntry>,
    ) -> Option<Self> {
        resolve_scalar(key, contributions, |a| format!("{a:?}"), conflicts)
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
