//! Core requirement trait implementations for deny.toml value types.

use core::cmp::Ordering;

use aqc_file_engine_core::{
    ConflictEntry, FileItemRequirement, Provenance, ResolvedRequirement, ScalarValue,
    compose_item_by,
};

use crate::requirement::value;

impl ScalarValue for value::DenyLintLevel {
    fn render(&self) -> String {
        self.as_str().to_owned()
    }
    fn compare_for_order(&self, _other: &Self) -> Option<Ordering> {
        None
    }
}

impl ScalarValue for value::DenyAdvisoryScope {
    fn render(&self) -> String {
        self.as_str().to_owned()
    }
    fn compare_for_order(&self, _other: &Self) -> Option<Ordering> {
        None
    }
}

impl ScalarValue for value::DenyGraphHighlight {
    fn render(&self) -> String {
        self.as_str().to_owned()
    }
    fn compare_for_order(&self, _other: &Self) -> Option<Ordering> {
        None
    }
}

impl ScalarValue for value::DenyGitSpec {
    fn render(&self) -> String {
        self.as_str().to_owned()
    }
    fn compare_for_order(&self, _other: &Self) -> Option<Ordering> {
        None
    }
}

impl ScalarValue for value::DenyNonEmptyString {
    fn render(&self) -> String {
        self.0.clone()
    }
    fn compare_for_order(&self, _other: &Self) -> Option<Ordering> {
        None
    }
}

impl ScalarValue for value::DenyPackageSpec {
    fn render(&self) -> String {
        self.0.clone()
    }
    fn compare_for_order(&self, _other: &Self) -> Option<Ordering> {
        None
    }
}

impl ScalarValue for value::DenyDuration {
    fn render(&self) -> String {
        self.0.clone()
    }
    fn compare_for_order(&self, _other: &Self) -> Option<Ordering> {
        None
    }
}

impl ScalarValue for value::DenyConfidenceThreshold {
    fn render(&self) -> String {
        self.as_str().to_owned()
    }
    fn compare_for_order(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl FileItemRequirement for value::DenyGraphTargetSpec {
    type Identity = String;
    fn merge_identity(&self) -> Self::Identity {
        self.as_str().to_owned()
    }
    fn compose_item(
        key: &str,
        items: Vec<(Provenance, (Self, String))>,
        conflicts: &mut Vec<ConflictEntry>,
    ) -> Option<ResolvedRequirement<Self, (Self, String)>> {
        compose_item_by(key, items, |item| item.clone(), conflicts)
    }
}

impl FileItemRequirement for value::DenyAdvisoryIgnoreSpec {
    type Identity = String;
    fn merge_identity(&self) -> Self::Identity {
        self.as_str().to_owned()
    }
    fn compose_item(
        key: &str,
        items: Vec<(Provenance, (Self, String))>,
        conflicts: &mut Vec<ConflictEntry>,
    ) -> Option<ResolvedRequirement<Self, (Self, String)>> {
        compose_item_by(key, items, |item| item.clone(), conflicts)
    }
}

impl FileItemRequirement for value::DenyLicenseException {
    type Identity = String;
    fn merge_identity(&self) -> Self::Identity {
        self.as_str().to_owned()
    }
    fn compose_item(
        key: &str,
        items: Vec<(Provenance, (Self, String))>,
        conflicts: &mut Vec<ConflictEntry>,
    ) -> Option<ResolvedRequirement<Self, (Self, String)>> {
        compose_item_by(key, items, |item| item.clone(), conflicts)
    }
}

impl FileItemRequirement for value::DenyLicenseClarification {
    type Identity = String;
    fn merge_identity(&self) -> Self::Identity {
        self.version().map_or_else(
            || self.as_str().to_owned(),
            |version| format!("{}@{version}", self.as_str()),
        )
    }
    fn compose_item(
        key: &str,
        items: Vec<(Provenance, (Self, String))>,
        conflicts: &mut Vec<ConflictEntry>,
    ) -> Option<ResolvedRequirement<Self, (Self, String)>> {
        compose_item_by(key, items, |item| item.clone(), conflicts)
    }
}

impl FileItemRequirement for value::DenyPackageReasonSpec {
    type Identity = String;
    fn merge_identity(&self) -> Self::Identity {
        self.as_str().to_owned()
    }
    fn compose_item(
        key: &str,
        items: Vec<(Provenance, (Self, String))>,
        conflicts: &mut Vec<ConflictEntry>,
    ) -> Option<ResolvedRequirement<Self, (Self, String)>> {
        compose_item_by(key, items, |item| item.clone(), conflicts)
    }
}

impl FileItemRequirement for value::DenyBanSpec {
    type Identity = String;
    fn merge_identity(&self) -> Self::Identity {
        self.as_str().to_owned()
    }
    fn compose_item(
        key: &str,
        items: Vec<(Provenance, (Self, String))>,
        conflicts: &mut Vec<ConflictEntry>,
    ) -> Option<ResolvedRequirement<Self, (Self, String)>> {
        compose_item_by(key, items, |item| item.clone(), conflicts)
    }
}

impl FileItemRequirement for value::DenyFeatureBanSpec {
    type Identity = String;
    fn merge_identity(&self) -> Self::Identity {
        self.as_str().to_owned()
    }
    fn compose_item(
        key: &str,
        items: Vec<(Provenance, (Self, String))>,
        conflicts: &mut Vec<ConflictEntry>,
    ) -> Option<ResolvedRequirement<Self, (Self, String)>> {
        compose_item_by(key, items, |item| item.clone(), conflicts)
    }
}

impl FileItemRequirement for value::DenySkipTreeSpec {
    type Identity = String;
    fn merge_identity(&self) -> Self::Identity {
        self.as_str().to_owned()
    }
    fn compose_item(
        key: &str,
        items: Vec<(Provenance, (Self, String))>,
        conflicts: &mut Vec<ConflictEntry>,
    ) -> Option<ResolvedRequirement<Self, (Self, String)>> {
        compose_item_by(key, items, |item| item.clone(), conflicts)
    }
}

impl FileItemRequirement for value::DenyBuildGlobSpec {
    type Identity = String;
    fn merge_identity(&self) -> Self::Identity {
        self.as_str().to_owned()
    }
    fn compose_item(
        key: &str,
        items: Vec<(Provenance, (Self, String))>,
        conflicts: &mut Vec<ConflictEntry>,
    ) -> Option<ResolvedRequirement<Self, (Self, String)>> {
        compose_item_by(key, items, |item| item.clone(), conflicts)
    }
}
