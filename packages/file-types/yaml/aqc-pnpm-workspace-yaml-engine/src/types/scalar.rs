//! Closed pnpm scalar values.

use std::cmp::Ordering;
use std::fmt;

use aqc_file_engine_core::ScalarValue;
use schemars::JsonSchema;
use serde::{Deserialize, Deserializer, Serialize};

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
pub enum PnpmOnFail {
    #[serde(rename = "download")]
    Download,
    #[serde(rename = "error")]
    Error,
    #[serde(rename = "warn")]
    Warn,
    #[serde(rename = "ignore")]
    Ignore,
}

impl fmt::Display for PnpmOnFail {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Download => "download",
            Self::Error => "error",
            Self::Warn => "warn",
            Self::Ignore => "ignore",
        })
    }
}

impl ScalarValue for PnpmOnFail {
    fn render(&self) -> String {
        self.to_string()
    }

    fn compare_for_order(&self, _other: &Self) -> Option<Ordering> {
        None
    }
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
pub enum PnpmTrustPolicy {
    #[serde(rename = "no-downgrade")]
    NoDowngrade,
    #[serde(rename = "off")]
    Off,
}

impl fmt::Display for PnpmTrustPolicy {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::NoDowngrade => "no-downgrade",
            Self::Off => "off",
        })
    }
}

impl ScalarValue for PnpmTrustPolicy {
    fn render(&self) -> String {
        self.to_string()
    }

    fn compare_for_order(&self, _other: &Self) -> Option<Ordering> {
        None
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, JsonSchema)]
pub struct PnpmReleaseAgeMinutes(u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PnpmReleaseAgeMinutesError {
    ExceedsJavaScriptSafeInteger,
}

impl fmt::Display for PnpmReleaseAgeMinutesError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ExceedsJavaScriptSafeInteger => {
                formatter.write_str("value exceeds JavaScript's safe integer maximum")
            }
        }
    }
}

impl core::error::Error for PnpmReleaseAgeMinutesError {}

impl PnpmReleaseAgeMinutes {
    /// Constructs a pnpm release age within JavaScript's safe integer range.
    ///
    /// # Errors
    ///
    /// Returns an error when the value exceeds 9,007,199,254,740,991.
    pub const fn new(value: u64) -> Result<Self, PnpmReleaseAgeMinutesError> {
        if value > 9_007_199_254_740_991 {
            Err(PnpmReleaseAgeMinutesError::ExceedsJavaScriptSafeInteger)
        } else {
            Ok(Self(value))
        }
    }

    #[must_use]
    pub const fn get(&self) -> u64 {
        self.0
    }
}

impl<'de> Deserialize<'de> for PnpmReleaseAgeMinutes {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = u64::deserialize(deserializer)?;
        Self::new(value).map_err(<D::Error as serde::de::Error>::custom)
    }
}

impl ScalarValue for PnpmReleaseAgeMinutes {
    fn render(&self) -> String {
        self.0.to_string()
    }

    fn compare_for_order(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
