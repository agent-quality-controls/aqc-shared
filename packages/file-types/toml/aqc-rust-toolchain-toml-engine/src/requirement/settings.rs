//! Rust toolchain TOML setting names.

use aqc_file_engine_core::{ConfigScalar, ScalarAssertion};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RustToolchainScalarSetting {
    Channel,
    Path,
    Profile,
}

impl RustToolchainScalarSetting {
    #[must_use]
    pub const fn file_key(self) -> &'static str {
        match self {
            Self::Channel => "channel",
            Self::Path => "path",
            Self::Profile => "profile",
        }
    }

    pub(super) fn scalar_assertion_is_legal(
        self,
        assertion: &ScalarAssertion<ConfigScalar>,
    ) -> bool {
        if matches!(
            assertion,
            ScalarAssertion::AtLeast(..) | ScalarAssertion::AtMost(..) | ScalarAssertion::Range(..)
        ) {
            return false;
        }
        match assertion {
            ScalarAssertion::Equals(ConfigScalar::Str(_), _)
            | ScalarAssertion::OneOf(_, _)
            | ScalarAssertion::Present(_)
            | ScalarAssertion::Absent(_) => true,
            ScalarAssertion::Equals(..)
            | ScalarAssertion::AtLeast(..)
            | ScalarAssertion::AtMost(..)
            | ScalarAssertion::Range(..) => false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RustToolchainListSetting {
    Components,
    Targets,
}

impl RustToolchainListSetting {
    #[must_use]
    pub const fn file_key(self) -> &'static str {
        match self {
            Self::Components => "components",
            Self::Targets => "targets",
        }
    }
}
