//! `[features]` table value payloads.

use std::collections::BTreeSet;

/// Enabled feature names for one `[features]` key.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FeatureMembers {
    pub members: BTreeSet<String>,
}
