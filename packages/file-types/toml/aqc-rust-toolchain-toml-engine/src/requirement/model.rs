//! Rust toolchain requirement model types.

#![allow(
    clippy::disallowed_types,
    reason = "`Any` is used only for EngineRequirement downcast dispatch."
)]

use core::any::Any;
use core::cmp::Ordering;
use core::fmt;
use std::path::{Path, PathBuf};

use aqc_file_engine_core::{
    EngineRequirement, ItemRequirements, KeyedItem, ListRequirements, ResolvedItemRequirements,
    ResolvedListRequirements, ResolvedRequirement, ScalarAssertion, ScalarValue,
};
use schemars::JsonSchema;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, JsonSchema)]
pub struct RustToolchainChannel(#[schemars(length(min = 1))] String);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RustToolchainValueError {
    Empty { field: &'static str },
    UnknownProfile { value: String },
    RelativePath { value: PathBuf },
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, JsonSchema)]
#[schemars(rename_all = "lowercase")]
pub enum RustToolchainProfile {
    #[default]
    Minimal,
    Default,
    Complete,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct RustToolchainPath(PathBuf);

impl RustToolchainChannel {
    pub fn new(value: impl Into<String>) -> Result<Self, RustToolchainValueError> {
        let value = value.into();
        if value.is_empty() {
            return Err(RustToolchainValueError::Empty { field: "channel" });
        }
        Ok(Self(value))
    }

    #[must_use]
    pub fn stable() -> Self {
        Self("stable".to_owned())
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for RustToolchainChannel {
    fn default() -> Self {
        Self::stable()
    }
}

impl RustToolchainProfile {
    pub fn parse(value: &str) -> Result<Self, RustToolchainValueError> {
        match value {
            "minimal" => Ok(Self::Minimal),
            "default" => Ok(Self::Default),
            "complete" => Ok(Self::Complete),
            "" => Err(RustToolchainValueError::Empty { field: "profile" }),
            other => Err(RustToolchainValueError::UnknownProfile {
                value: other.to_owned(),
            }),
        }
    }

    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Minimal => "minimal",
            Self::Default => "default",
            Self::Complete => "complete",
        }
    }
}

impl RustToolchainPath {
    pub fn new(value: impl Into<PathBuf>) -> Result<Self, RustToolchainValueError> {
        let value = value.into();
        if value.as_os_str().is_empty() {
            return Err(RustToolchainValueError::Empty { field: "path" });
        }
        if !value.is_absolute() {
            return Err(RustToolchainValueError::RelativePath { value });
        }
        Ok(Self(value))
    }

    #[must_use]
    pub fn as_path(&self) -> &Path {
        &self.0
    }
}

impl Serialize for RustToolchainChannel {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for RustToolchainChannel {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Self::new(String::deserialize(deserializer)?).map_err(serde::de::Error::custom)
    }
}

impl Serialize for RustToolchainProfile {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for RustToolchainProfile {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Self::parse(&String::deserialize(deserializer)?).map_err(serde::de::Error::custom)
    }
}

impl Serialize for RustToolchainPath {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.0.to_string_lossy())
    }
}

impl<'de> Deserialize<'de> for RustToolchainPath {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Self::new(PathBuf::from(String::deserialize(deserializer)?))
            .map_err(serde::de::Error::custom)
    }
}

impl fmt::Display for RustToolchainValueError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty { field } => write!(f, "{field} must not be empty"),
            Self::UnknownProfile { value } => write!(f, "unknown rustup profile {value}"),
            Self::RelativePath { value } => write!(f, "path must be absolute: {}", value.display()),
        }
    }
}

impl ScalarValue for RustToolchainChannel {
    fn render(&self) -> String {
        self.0.clone()
    }

    fn compare_for_order(&self, _other: &Self) -> Option<Ordering> {
        None
    }
}

impl ScalarValue for RustToolchainProfile {
    fn render(&self) -> String {
        self.as_str().to_owned()
    }

    fn compare_for_order(&self, _other: &Self) -> Option<Ordering> {
        None
    }
}

impl ScalarValue for RustToolchainPath {
    fn render(&self) -> String {
        self.0.to_string_lossy().into_owned()
    }

    fn compare_for_order(&self, _other: &Self) -> Option<Ordering> {
        None
    }
}

#[derive(Debug, Clone, Default)]
pub struct RustToolchainTomlRequirements {
    pub channel: Option<ScalarAssertion<RustToolchainChannel>>,
    pub path: Option<ScalarAssertion<RustToolchainPath>>,
    pub profile: Option<ScalarAssertion<RustToolchainProfile>>,
    pub components: ListRequirements,
    pub targets: ListRequirements,
    pub toolchain_keys: ItemRequirements<KeyedItem<()>>,
}

#[derive(Debug, Clone, Default)]
#[rustfmt::skip]
pub struct ResolvedRustToolchainTomlRequirements {
    pub(crate) channel: Option<ResolvedRequirement<ScalarAssertion<RustToolchainChannel>, ScalarAssertion<RustToolchainChannel>>>,
    pub(crate) path: Option<ResolvedRequirement<ScalarAssertion<RustToolchainPath>, ScalarAssertion<RustToolchainPath>>>,
    pub(crate) profile: Option<ResolvedRequirement<ScalarAssertion<RustToolchainProfile>, ScalarAssertion<RustToolchainProfile>>>,
    pub(crate) components: ResolvedListRequirements,
    pub(crate) targets: ResolvedListRequirements,
    pub(crate) toolchain_keys: ResolvedItemRequirements<KeyedItem<()>>,
}

impl ResolvedRustToolchainTomlRequirements {
    #[must_use]
    pub const fn channel(
        &self,
    ) -> Option<
        &ResolvedRequirement<
            ScalarAssertion<RustToolchainChannel>,
            ScalarAssertion<RustToolchainChannel>,
        >,
    > {
        self.channel.as_ref()
    }

    #[must_use]
    pub const fn path(
        &self,
    ) -> Option<
        &ResolvedRequirement<
            ScalarAssertion<RustToolchainPath>,
            ScalarAssertion<RustToolchainPath>,
        >,
    > {
        self.path.as_ref()
    }

    #[must_use]
    pub const fn profile(
        &self,
    ) -> Option<
        &ResolvedRequirement<
            ScalarAssertion<RustToolchainProfile>,
            ScalarAssertion<RustToolchainProfile>,
        >,
    > {
        self.profile.as_ref()
    }

    #[must_use]
    pub const fn components(&self) -> &ResolvedListRequirements {
        &self.components
    }

    #[must_use]
    pub const fn targets(&self) -> &ResolvedListRequirements {
        &self.targets
    }

    #[must_use]
    pub const fn toolchain_keys(&self) -> &ResolvedItemRequirements<KeyedItem<()>> {
        &self.toolchain_keys
    }
}

impl EngineRequirement for RustToolchainTomlRequirements {
    fn engine_id(&self) -> &'static str {
        crate::ENGINE_ID
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
