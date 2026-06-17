//! Dependency-table scopes and entry payloads.

use std::collections::BTreeSet;
use std::fmt;

use aqc_file_engine_core::{
    ConflictEntry, FileItemRequirement, PatternBanRequirement, Provenance, ResolvedRequirement,
    compose_optional_field, compose_string_set,
};

/// Which dependency table kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DependencyKind {
    Normal,
    Dev,
    Build,
}

/// Which dependency table: kind plus optional target.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct DependencyScope {
    pub kind: DependencyKind,
    pub target: Option<String>,
}

impl DependencyScope {
    /// The in-file table path this scope addresses.
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

/// Partial dependency entry spec. Unset fields are unconstrained.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DependencySpec {
    pub version: Option<String>,
    pub features: BTreeSet<String>,
    pub default_features: Option<bool>,
    pub optional: Option<bool>,
    pub workspace: Option<bool>,
    pub path: Option<String>,
    pub git: Option<String>,
    pub branch: Option<String>,
    pub tag: Option<String>,
    pub rev: Option<String>,
    pub registry: Option<String>,
    pub package: Option<String>,
}

/// A dependency file-item requirement.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DependencyRequirement {
    pub file_key: Option<String>,
    pub value: DependencySpec,
}

/// Package-name glob used only for forbidden dependency package families.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DependencyPackagePattern {
    pub pattern: String,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum DependencyIdentity {
    Package(String),
    LocalKey(String),
    Invalid,
}

impl fmt::Display for DependencyIdentity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Package(package) => write!(f, "{package}"),
            Self::LocalKey(file_key) => write!(f, "{file_key}"),
            Self::Invalid => write!(f, "<invalid>"),
        }
    }
}

impl PatternBanRequirement for DependencyPackagePattern {
    type Identity = String;

    fn merge_identity(&self) -> Self::Identity {
        self.pattern.clone()
    }

    fn render(&self) -> String {
        self.pattern.clone()
    }
}

impl DependencySpec {
    /// True when the spec names where the code comes from.
    #[must_use]
    pub const fn has_source(&self) -> bool {
        self.version.is_some()
            || self.path.is_some()
            || self.git.is_some()
            || self.workspace.is_some()
    }
}

impl FileItemRequirement for DependencyRequirement {
    type Identity = DependencyIdentity;

    fn merge_identity(&self) -> Self::Identity {
        if let Some(package) = self.value.package.clone() {
            return DependencyIdentity::Package(package);
        }
        if let Some(file_key) = self.file_key.clone() {
            return DependencyIdentity::LocalKey(file_key);
        }
        DependencyIdentity::Invalid
    }

    fn compose_item(
        key: &str,
        items: Vec<(Provenance, (Self, String))>,
        conflicts: &mut Vec<ConflictEntry>,
    ) -> Option<ResolvedRequirement<Self, (Self, String)>> {
        let file_keys = items
            .iter()
            .filter_map(|(_, (requirement, _))| requirement.file_key.clone())
            .collect::<BTreeSet<_>>();
        if file_keys.len() > 1 {
            conflicts.push(ConflictEntry {
                key: format!("{key}.file_key"),
                reason: "dependency-package-multiple-file-keys".to_owned(),
                contributors: items
                    .iter()
                    .filter_map(|(prov, (requirement, _))| {
                        requirement
                            .file_key
                            .as_ref()
                            .map(|file_key| (prov.clone(), file_key.clone()))
                    })
                    .collect(),
            });
            return None;
        }
        let specs = items
            .iter()
            .map(|(prov, (requirement, _))| (prov.clone(), requirement.value.clone()))
            .collect::<Vec<_>>();
        let merged = DependencySpec {
            version: compose_optional_field(
                &format!("{key}.version"),
                specs
                    .iter()
                    .map(|(prov, spec)| (prov.clone(), spec.version.clone()))
                    .collect(),
                Clone::clone,
                conflicts,
            ),
            features: compose_string_set(
                specs
                    .iter()
                    .map(|(_, spec)| spec.features.clone())
                    .collect(),
            ),
            default_features: compose_optional_field(
                &format!("{key}.default-features"),
                specs
                    .iter()
                    .map(|(prov, spec)| (prov.clone(), spec.default_features))
                    .collect(),
                bool::to_string,
                conflicts,
            ),
            optional: compose_optional_field(
                &format!("{key}.optional"),
                specs
                    .iter()
                    .map(|(prov, spec)| (prov.clone(), spec.optional))
                    .collect(),
                bool::to_string,
                conflicts,
            ),
            workspace: compose_optional_field(
                &format!("{key}.workspace"),
                specs
                    .iter()
                    .map(|(prov, spec)| (prov.clone(), spec.workspace))
                    .collect(),
                bool::to_string,
                conflicts,
            ),
            path: compose_optional_field(
                &format!("{key}.path"),
                specs
                    .iter()
                    .map(|(prov, spec)| (prov.clone(), spec.path.clone()))
                    .collect(),
                Clone::clone,
                conflicts,
            ),
            git: compose_optional_field(
                &format!("{key}.git"),
                specs
                    .iter()
                    .map(|(prov, spec)| (prov.clone(), spec.git.clone()))
                    .collect(),
                Clone::clone,
                conflicts,
            ),
            branch: compose_optional_field(
                &format!("{key}.branch"),
                specs
                    .iter()
                    .map(|(prov, spec)| (prov.clone(), spec.branch.clone()))
                    .collect(),
                Clone::clone,
                conflicts,
            ),
            tag: compose_optional_field(
                &format!("{key}.tag"),
                specs
                    .iter()
                    .map(|(prov, spec)| (prov.clone(), spec.tag.clone()))
                    .collect(),
                Clone::clone,
                conflicts,
            ),
            rev: compose_optional_field(
                &format!("{key}.rev"),
                specs
                    .iter()
                    .map(|(prov, spec)| (prov.clone(), spec.rev.clone()))
                    .collect(),
                Clone::clone,
                conflicts,
            ),
            registry: compose_optional_field(
                &format!("{key}.registry"),
                specs
                    .iter()
                    .map(|(prov, spec)| (prov.clone(), spec.registry.clone()))
                    .collect(),
                Clone::clone,
                conflicts,
            ),
            package: compose_optional_field(
                &format!("{key}.package"),
                specs
                    .iter()
                    .map(|(prov, spec)| (prov.clone(), spec.package.clone()))
                    .collect(),
                Clone::clone,
                conflicts,
            ),
        };
        Some(ResolvedRequirement {
            merged: DependencyRequirement {
                file_key: items
                    .iter()
                    .find_map(|(_, (requirement, _))| requirement.file_key.clone()),
                value: merged,
            },
            collected: items,
        })
    }
}
