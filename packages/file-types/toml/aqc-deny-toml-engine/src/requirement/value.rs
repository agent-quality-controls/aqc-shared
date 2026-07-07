//! Deny TOML scalar and item value types.

use std::cmp::Ordering;
use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

mod value_impls;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DenyTomlValueError {
    Empty {
        field: &'static str,
    },
    Invalid {
        field: &'static str,
        value: String,
        reason: &'static str,
    },
    UnknownEnum {
        field: &'static str,
        value: String,
    },
    OverlappingFeatures {
        package: String,
        feature: String,
    },
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct DenyConfidenceThreshold {
    text: String,
    millionths: u32,
}

impl PartialEq for DenyConfidenceThreshold {
    fn eq(&self, other: &Self) -> bool {
        self.millionths == other.millionths
    }
}

impl Eq for DenyConfidenceThreshold {}

impl PartialOrd for DenyConfidenceThreshold {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for DenyConfidenceThreshold {
    fn cmp(&self, other: &Self) -> Ordering {
        self.millionths.cmp(&other.millionths)
    }
}

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

impl DenyDuration {
    pub fn new(value: impl Into<String>) -> Result<Self, DenyTomlValueError> {
        let value = value.into();
        if value.is_empty() {
            return Err(DenyTomlValueError::Empty { field: "duration" });
        }
        if !value.starts_with('P') {
            return Err(DenyTomlValueError::Invalid {
                field: "duration",
                value,
                reason: "duration must use cargo-deny ISO-8601 form such as P90D",
            });
        }
        Ok(Self(value))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl DenyConfidenceThreshold {
    pub fn new(value: impl Into<String>) -> Result<Self, DenyTomlValueError> {
        let value = value.into();
        if value.is_empty() {
            return Err(DenyTomlValueError::Empty {
                field: "confidence-threshold",
            });
        }
        let millionths = parse_confidence_millionths(&value)?;
        Ok(Self {
            text: canonical_confidence_text(&value),
            millionths,
        })
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.text
    }

    #[must_use]
    pub fn as_f64(&self) -> f64 {
        self.text.parse::<f64>().unwrap_or(0.0)
    }
}

impl TryFrom<String> for DenyConfidenceThreshold {
    type Error = DenyTomlValueError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<DenyConfidenceThreshold> for String {
    fn from(value: DenyConfidenceThreshold) -> Self {
        value.text
    }
}

fn parse_confidence_millionths(value: &str) -> Result<u32, DenyTomlValueError> {
    let Some((whole, fraction)) = value.split_once('.') else {
        return match value {
            "0" => Ok(0),
            "1" => Ok(1_000_000),
            _ => Err(invalid_confidence(value)),
        };
    };
    if fraction.is_empty()
        || fraction.len() > 6
        || !fraction.bytes().all(|byte| byte.is_ascii_digit())
    {
        return Err(invalid_confidence(value));
    }
    let padded = format!("{fraction:0<6}");
    let fraction_value = padded
        .parse::<u32>()
        .map_err(|_| invalid_confidence(value))?;
    match whole {
        "0" => Ok(fraction_value),
        "1" if fraction_value == 0 => Ok(1_000_000),
        _ => Err(invalid_confidence(value)),
    }
}

fn canonical_confidence_text(value: &str) -> String {
    let mut out = value.to_owned();
    if out.contains('.') {
        while out.ends_with('0') {
            let _ = out.pop();
        }
        if out.ends_with('.') {
            out.push('0');
        }
    }
    out
}

fn invalid_confidence(value: &str) -> DenyTomlValueError {
    DenyTomlValueError::Invalid {
        field: "confidence-threshold",
        value: value.to_owned(),
        reason: "confidence-threshold must be a number from 0.0 through 1.0",
    }
}
