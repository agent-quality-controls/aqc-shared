#![allow(
    clippy::disallowed_types,
    reason = "Any is required by the engine requirement downcast contract."
)]

use core::any::Any;
use core::cmp::Ordering;
use std::collections::BTreeMap;

use aqc_file_engine_core::merge::ResolvedMap;
use aqc_file_engine_core::{EngineRequirement, ResolvedRequirement, ScalarAssertion, ScalarValue};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema)]
pub enum PackageManagerOnFail {
    #[serde(rename = "download")]
    #[schemars(rename = "download")]
    Download,
    #[serde(rename = "error")]
    #[schemars(rename = "error")]
    Error,
    #[serde(rename = "warn")]
    #[schemars(rename = "warn")]
    Warn,
    #[serde(rename = "ignore")]
    #[schemars(rename = "ignore")]
    Ignore,
}

impl PackageManagerOnFail {
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Download => "download",
            Self::Error => "error",
            Self::Warn => "warn",
            Self::Ignore => "ignore",
        }
    }

    #[must_use]
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "download" => Some(Self::Download),
            "error" => Some(Self::Error),
            "warn" => Some(Self::Warn),
            "ignore" => Some(Self::Ignore),
            _ => None,
        }
    }
}

impl ScalarValue for PackageManagerOnFail {
    fn render(&self) -> String {
        self.as_str().to_owned()
    }

    fn compare_for_order(&self, _other: &Self) -> Option<Ordering> {
        None
    }
}

#[derive(Debug, Clone, Default)]
pub struct PackageJsonRequirements {
    pub package_manager: Option<ScalarAssertion<String>>,
    pub dev_engines_package_manager: DevEnginePackageManagerRequirements,
    pub scripts: BTreeMap<String, ScalarAssertion<String>>,
    pub dev_dependencies: BTreeMap<String, ScalarAssertion<String>>,
}

#[derive(Debug, Clone, Default)]
pub struct DevEnginePackageManagerRequirements {
    pub name: Option<ScalarAssertion<String>>,
    pub version: Option<ScalarAssertion<String>>,
    pub on_fail: Option<ScalarAssertion<PackageManagerOnFail>>,
}

#[derive(Debug, Clone)]
pub struct ResolvedPackageJsonRequirements {
    pub(crate) package_manager:
        Option<ResolvedRequirement<ScalarAssertion<String>, ScalarAssertion<String>>>,
    pub(crate) dev_engines_package_manager: ResolvedDevEnginePackageManagerRequirements,
    pub(crate) scripts: ResolvedMap<String, ScalarAssertion<String>>,
    pub(crate) dev_dependencies: ResolvedMap<String, ScalarAssertion<String>>,
}

#[derive(Debug, Clone)]
#[rustfmt::skip]
pub struct ResolvedDevEnginePackageManagerRequirements {
    pub(crate) name: Option<ResolvedRequirement<ScalarAssertion<String>, ScalarAssertion<String>>>,
    pub(crate) version: Option<ResolvedRequirement<ScalarAssertion<String>, ScalarAssertion<String>>>,
    pub(crate) on_fail: Option<ResolvedRequirement<ScalarAssertion<PackageManagerOnFail>, ScalarAssertion<PackageManagerOnFail>>>,
}

impl ResolvedPackageJsonRequirements {
    #[must_use]
    pub const fn package_manager(
        &self,
    ) -> Option<&ResolvedRequirement<ScalarAssertion<String>, ScalarAssertion<String>>> {
        self.package_manager.as_ref()
    }

    #[must_use]
    pub const fn dev_engines_package_manager(
        &self,
    ) -> &ResolvedDevEnginePackageManagerRequirements {
        &self.dev_engines_package_manager
    }

    #[must_use]
    pub const fn scripts(&self) -> &ResolvedMap<String, ScalarAssertion<String>> {
        &self.scripts
    }

    #[must_use]
    pub const fn dev_dependencies(&self) -> &ResolvedMap<String, ScalarAssertion<String>> {
        &self.dev_dependencies
    }
}

impl ResolvedDevEnginePackageManagerRequirements {
    #[must_use]
    pub const fn name(
        &self,
    ) -> Option<&ResolvedRequirement<ScalarAssertion<String>, ScalarAssertion<String>>> {
        self.name.as_ref()
    }

    #[must_use]
    pub const fn version(
        &self,
    ) -> Option<&ResolvedRequirement<ScalarAssertion<String>, ScalarAssertion<String>>> {
        self.version.as_ref()
    }

    #[must_use]
    pub const fn on_fail(
        &self,
    ) -> Option<
        &ResolvedRequirement<
            ScalarAssertion<PackageManagerOnFail>,
            ScalarAssertion<PackageManagerOnFail>,
        >,
    > {
        self.on_fail.as_ref()
    }
}

impl EngineRequirement for PackageJsonRequirements {
    fn engine_id(&self) -> &'static str {
        crate::ENGINE_ID
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
