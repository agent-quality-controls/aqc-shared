//! Deny TOML scalar and item value types.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

mod value_impls;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DenyTomlValueError {
    Empty { field: &'static str },
    UnknownEnum { field: &'static str, value: String },
    OverlappingFeatures { package: String, feature: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum DenyLintLevel {
    Allow,
    Warn,
    Deny,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum DenyAdvisoryScope {
    All,
    Workspace,
    Transitive,
    None,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum DenyGraphHighlight {
    All,
    SimplestPath,
    LowestVersion,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum DenyGitSpec {
    Any,
    Branch,
    Tag,
    Rev,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct DenyNonEmptyString(String);

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct DenyPackageSpec(String);

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct DenyDuration(String);

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct DenyConfidenceThreshold(String);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DenyGraphTargetSpec {
    triple: DenyNonEmptyString,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum DenyAdvisoryIgnoreIdentity {
    Id(DenyNonEmptyString),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DenyAdvisoryIgnoreSpec {
    identity: DenyAdvisoryIgnoreIdentity,
    reason: Option<DenyNonEmptyString>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DenyLicenseException {
    package: DenyPackageSpec,
    license: DenyNonEmptyString,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DenyLicenseFile {
    path: DenyNonEmptyString,
    hash: DenyNonEmptyString,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DenyLicenseClarification {
    package: DenyPackageSpec,
    version: Option<DenyNonEmptyString>,
    expression: DenyNonEmptyString,
    license_files: Vec<DenyLicenseFile>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DenyPackageReasonSpec {
    package: DenyPackageSpec,
    reason: Option<DenyNonEmptyString>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DenyBanSpec {
    package: DenyPackageSpec,
    reason: Option<DenyNonEmptyString>,
    wrappers: Vec<DenyPackageSpec>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DenyFeatureBanSpec {
    package: DenyPackageSpec,
    allowed_features: BTreeSet<DenyNonEmptyString>,
    forbidden_features: BTreeSet<DenyNonEmptyString>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DenySkipTreeSpec {
    package: DenyPackageSpec,
    depth: Option<u64>,
    reason: Option<DenyNonEmptyString>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DenyBuildGlobSpec {
    glob: DenyNonEmptyString,
    reason: Option<DenyNonEmptyString>,
}

macro_rules! impl_closed_enum {
    ($type_name:ident, $field:literal, [$(($variant:ident, $text:literal)),+ $(,)?]) => {
        impl $type_name {
            pub fn parse(value: &str) -> Result<Self, DenyTomlValueError> {
                match value {
                    $($text => Ok(Self::$variant),)+
                    "" => Err(DenyTomlValueError::Empty { field: $field }),
                    other => Err(DenyTomlValueError::UnknownEnum {
                        field: $field,
                        value: other.to_owned(),
                    }),
                }
            }

            #[must_use]
            pub fn as_str(&self) -> &'static str {
                match self {
                    $(Self::$variant => $text,)+
                }
            }
        }

    };
}

impl_closed_enum!(
    DenyLintLevel,
    "lint-level",
    [(Allow, "allow"), (Warn, "warn"), (Deny, "deny")]
);
impl_closed_enum!(
    DenyAdvisoryScope,
    "advisory-scope",
    [
        (All, "all"),
        (Workspace, "workspace"),
        (Transitive, "transitive"),
        (None, "none")
    ]
);
impl_closed_enum!(
    DenyGraphHighlight,
    "graph-highlight",
    [
        (All, "all"),
        (SimplestPath, "simplest-path"),
        (LowestVersion, "lowest-version")
    ]
);
impl_closed_enum!(
    DenyGitSpec,
    "git-spec",
    [(Any, "any"), (Branch, "branch"), (Tag, "tag"), (Rev, "rev")]
);

macro_rules! impl_text_wrapper {
    ($type_name:ident, $field:literal) => {
        impl $type_name {
            pub fn new(value: impl Into<String>) -> Result<Self, DenyTomlValueError> {
                let value = value.into();
                if value.is_empty() {
                    return Err(DenyTomlValueError::Empty { field: $field });
                }
                Ok(Self(value))
            }

            #[must_use]
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }
    };
}

impl_text_wrapper!(DenyNonEmptyString, "text");
impl_text_wrapper!(DenyPackageSpec, "package");
impl_text_wrapper!(DenyDuration, "duration");
impl_text_wrapper!(DenyConfidenceThreshold, "confidence-threshold");
