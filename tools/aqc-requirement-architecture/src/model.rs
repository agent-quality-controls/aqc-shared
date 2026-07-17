use std::fmt::{Display, Formatter};
use std::path::PathBuf;

use serde::Serialize;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum RequirementKind {
    Engine,
    Adapter,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ViolationCode {
    SemanticClosureField,
    AdapterMembershipConstruction,
    NonCanonicalRequirementRoot,
    ReimplementedCoreVocabulary,
    UninspectableRequirementImport,
    UninspectableRequirementMacro,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct MembershipField {
    pub name: String,
    pub rust_type: String,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct RequirementRoot {
    pub crate_name: String,
    pub kind: RequirementKind,
    pub manifest: String,
    pub membership_fields: Vec<MembershipField>,
    pub name: String,
    pub repository_root: String,
    pub source: String,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct ArchitectureViolation {
    pub code: ViolationCode,
    pub crate_name: String,
    pub message: String,
    pub source: String,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize)]
pub struct ArchitectureReport {
    pub roots: Vec<RequirementRoot>,
    pub violations: Vec<ArchitectureViolation>,
}

#[derive(Debug)]
pub enum ArchitectureError {
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
    Metadata {
        manifest: PathBuf,
        message: String,
    },
    Parse {
        path: PathBuf,
        source: syn::Error,
    },
}

impl Display for ArchitectureError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io { path, source } => write!(formatter, "{}: {source}", path.display()),
            Self::Metadata { manifest, message } => {
                write!(formatter, "{}: {message}", manifest.display())
            }
            Self::Parse { path, source } => write!(formatter, "{}: {source}", path.display()),
        }
    }
}

impl std::error::Error for ArchitectureError {}
