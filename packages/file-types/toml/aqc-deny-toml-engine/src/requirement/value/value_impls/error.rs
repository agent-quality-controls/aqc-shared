//! Display implementation for deny.toml value errors.

use core::fmt;

use super::super::DenyTomlValueError;

impl fmt::Display for DenyTomlValueError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty { field } => write!(f, "{field} must not be empty"),
            Self::Invalid {
                field,
                value,
                reason,
            } => write!(f, "invalid {field} value {value}: {reason}"),
            Self::UnknownEnum { field, value } => {
                write!(f, "unknown {field} value {value}")
            }
            Self::OverlappingFeatures { package, feature } => {
                write!(f, "{package} allows and denies feature {feature}")
            }
        }
    }
}
