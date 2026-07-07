//! Constructors and accessors for deny.toml value types.

use std::collections::BTreeSet;

use super::super::{
    DenyAdvisoryIgnoreIdentity, DenyAdvisoryIgnoreSpec, DenyBanSpec, DenyBuildGlobSpec,
    DenyFeatureBanSpec, DenyGraphTargetSpec, DenyLicenseClarification, DenyLicenseException,
    DenyLicenseFile, DenyNonEmptyString, DenyPackageReasonSpec, DenyPackageSpec, DenySkipTreeSpec,
    DenyTomlValueError,
};

impl DenyGraphTargetSpec {
    pub fn new(triple: impl Into<String>) -> Result<Self, DenyTomlValueError> {
        Ok(Self {
            triple: DenyNonEmptyString::new(triple)?,
        })
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        self.triple.as_str()
    }
}

impl DenyAdvisoryIgnoreIdentity {
    pub fn new(value: impl Into<String>) -> Result<Self, DenyTomlValueError> {
        Ok(Self::Id(DenyNonEmptyString::new(value)?))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        match self {
            Self::Id(value) => value.as_str(),
        }
    }
}

impl DenyAdvisoryIgnoreSpec {
    pub fn new(value: impl Into<String>) -> Result<Self, DenyTomlValueError> {
        Ok(Self {
            identity: DenyAdvisoryIgnoreIdentity::new(value)?,
            reason: None,
        })
    }

    pub fn with_reason(
        value: impl Into<String>,
        reason: impl Into<String>,
    ) -> Result<Self, DenyTomlValueError> {
        Ok(Self {
            identity: DenyAdvisoryIgnoreIdentity::new(value)?,
            reason: Some(DenyNonEmptyString::new(reason)?),
        })
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        self.identity.as_str()
    }

    #[must_use]
    pub fn reason(&self) -> Option<&str> {
        self.reason.as_ref().map(DenyNonEmptyString::as_str)
    }
}

impl DenyLicenseException {
    pub fn new(
        package: impl Into<String>,
        license: impl Into<String>,
    ) -> Result<Self, DenyTomlValueError> {
        Ok(Self {
            package: DenyPackageSpec::new(package)?,
            license: DenyNonEmptyString::new(license)?,
        })
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        self.package.as_str()
    }

    #[must_use]
    pub fn license(&self) -> &str {
        self.license.as_str()
    }
}

impl DenyLicenseFile {
    pub fn new(
        path: impl Into<String>,
        hash: impl Into<String>,
    ) -> Result<Self, DenyTomlValueError> {
        Ok(Self {
            path: DenyNonEmptyString::new(path)?,
            hash: DenyNonEmptyString::new(hash)?,
        })
    }

    #[must_use]
    pub fn path(&self) -> &str {
        self.path.as_str()
    }

    #[must_use]
    pub fn hash(&self) -> &str {
        self.hash.as_str()
    }
}

impl DenyLicenseClarification {
    pub fn new(
        package: impl Into<String>,
        expression: impl Into<String>,
    ) -> Result<Self, DenyTomlValueError> {
        Ok(Self {
            package: DenyPackageSpec::new(package)?,
            version: None,
            expression: DenyNonEmptyString::new(expression)?,
            license_files: Vec::new(),
        })
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        self.package.as_str()
    }

    #[must_use]
    pub fn version(&self) -> Option<&str> {
        self.version.as_ref().map(DenyNonEmptyString::as_str)
    }

    #[must_use]
    pub fn expression(&self) -> &str {
        self.expression.as_str()
    }

    #[must_use]
    pub fn license_files(&self) -> &[DenyLicenseFile] {
        &self.license_files
    }
}

impl DenyPackageReasonSpec {
    pub fn new(package: impl Into<String>) -> Result<Self, DenyTomlValueError> {
        Ok(Self {
            package: DenyPackageSpec::new(package)?,
            reason: None,
        })
    }

    pub fn with_reason(
        package: impl Into<String>,
        reason: impl Into<String>,
    ) -> Result<Self, DenyTomlValueError> {
        Ok(Self {
            package: DenyPackageSpec::new(package)?,
            reason: Some(DenyNonEmptyString::new(reason)?),
        })
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        self.package.as_str()
    }

    #[must_use]
    pub fn reason(&self) -> Option<&str> {
        self.reason.as_ref().map(DenyNonEmptyString::as_str)
    }
}

impl DenyBanSpec {
    pub fn new(package: impl Into<String>) -> Result<Self, DenyTomlValueError> {
        Ok(Self {
            package: DenyPackageSpec::new(package)?,
            reason: None,
            wrappers: Vec::new(),
        })
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        self.package.as_str()
    }

    #[must_use]
    pub fn reason(&self) -> Option<&str> {
        self.reason.as_ref().map(DenyNonEmptyString::as_str)
    }

    #[must_use]
    pub fn wrappers(&self) -> &[DenyPackageSpec] {
        &self.wrappers
    }
}

impl DenyFeatureBanSpec {
    pub fn new(
        package: impl Into<String>,
        allowed_features: BTreeSet<DenyNonEmptyString>,
        forbidden_features: BTreeSet<DenyNonEmptyString>,
    ) -> Result<Self, DenyTomlValueError> {
        let package = DenyPackageSpec::new(package)?;
        if let Some(feature) = allowed_features.intersection(&forbidden_features).next() {
            return Err(DenyTomlValueError::OverlappingFeatures {
                package: package.as_str().to_owned(),
                feature: feature.as_str().to_owned(),
            });
        }
        Ok(Self {
            package,
            allowed_features,
            forbidden_features,
        })
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        self.package.as_str()
    }

    #[must_use]
    pub fn allowed_features(&self) -> &BTreeSet<DenyNonEmptyString> {
        &self.allowed_features
    }

    #[must_use]
    pub fn forbidden_features(&self) -> &BTreeSet<DenyNonEmptyString> {
        &self.forbidden_features
    }
}

impl DenySkipTreeSpec {
    pub fn new(package: impl Into<String>) -> Result<Self, DenyTomlValueError> {
        Ok(Self {
            package: DenyPackageSpec::new(package)?,
            depth: None,
            reason: None,
        })
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        self.package.as_str()
    }

    #[must_use]
    pub const fn depth(&self) -> Option<u64> {
        self.depth
    }

    #[must_use]
    pub fn reason(&self) -> Option<&str> {
        self.reason.as_ref().map(DenyNonEmptyString::as_str)
    }
}

impl DenyBuildGlobSpec {
    pub fn new(glob: impl Into<String>) -> Result<Self, DenyTomlValueError> {
        Ok(Self {
            glob: DenyNonEmptyString::new(glob)?,
            reason: None,
        })
    }

    pub fn with_reason(
        glob: impl Into<String>,
        reason: impl Into<String>,
    ) -> Result<Self, DenyTomlValueError> {
        Ok(Self {
            glob: DenyNonEmptyString::new(glob)?,
            reason: Some(DenyNonEmptyString::new(reason)?),
        })
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        self.glob.as_str()
    }

    #[must_use]
    pub fn reason(&self) -> Option<&str> {
        self.reason.as_ref().map(DenyNonEmptyString::as_str)
    }
}
