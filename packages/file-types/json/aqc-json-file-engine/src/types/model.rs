#![allow(
    clippy::disallowed_types,
    reason = "EngineRequirement uses Any for erased dispatch."
)]
#![expect(
    clippy::type_complexity,
    reason = "Public requirement fields intentionally expose canonical core generic vocabulary."
)]

use core::any::Any;
use std::collections::BTreeMap;

use aqc_file_engine_core::{
    ConfigScalar, EngineRequirement, FindingKey, ForbiddenGlobRequirement,
    ForbiddenGlobRequirements, ItemRequirements, KeyedItem, ListRequirements,
    ResolvedForbiddenGlobRequirements, ResolvedItemRequirements, ResolvedListRequirements,
    ScalarAssertion, merge::ResolvedMap,
};
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct JsonPath {
    components: Vec<String>,
}

impl JsonPath {
    #[must_use]
    pub const fn root() -> Self {
        Self {
            components: Vec::new(),
        }
    }

    #[must_use]
    pub fn new(first: impl Into<String>) -> Self {
        Self {
            components: vec![first.into()],
        }
    }

    #[must_use]
    pub fn child(mut self, component: impl Into<String>) -> Self {
        self.components.push(component.into());
        self
    }

    pub fn components(&self) -> impl Iterator<Item = &str> {
        self.components.iter().map(String::as_str)
    }

    #[must_use]
    pub fn pointer(&self) -> String {
        self.components
            .iter()
            .fold(String::new(), |mut out, component| {
                out.push('/');
                out.push_str(&component.replace('~', "~0").replace('/', "~1"));
                out
            })
    }

    #[must_use]
    pub fn selector(&self) -> String {
        self.components
            .last()
            .cloned()
            .unwrap_or_else(|| "$".to_owned())
    }

    pub(crate) fn finding_key(&self) -> String {
        if self.components.is_empty() {
            "$".to_owned()
        } else {
            self.pointer()
        }
    }
}

impl FindingKey for JsonPath {
    fn key(&self) -> String {
        self.finding_key()
    }

    fn child_key(&self, child: &str) -> String {
        self.clone().child(child).finding_key()
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct JsonStringGlob {
    pub glob: String,
}

impl ForbiddenGlobRequirement for JsonStringGlob {
    type Identity = String;

    fn merge_identity(&self) -> Self::Identity {
        self.glob.clone()
    }

    fn render(&self) -> String {
        self.glob.clone()
    }
}

#[derive(Debug, Clone, Default)]
pub struct JsonFileRequirements {
    pub scalar_values: BTreeMap<JsonPath, ScalarAssertion<ConfigScalar>>,
    pub string_lists: BTreeMap<JsonPath, ListRequirements>,
    pub forbidden_string_list_globs: BTreeMap<JsonPath, ForbiddenGlobRequirements<JsonStringGlob>>,
    pub object_keys: BTreeMap<JsonPath, ItemRequirements<KeyedItem<()>>>,
}

#[derive(Debug, Clone, Default)]
pub struct ResolvedJsonFileRequirements {
    pub(crate) scalar_values: ResolvedMap<JsonPath, ScalarAssertion<ConfigScalar>>,
    pub(crate) string_lists: BTreeMap<JsonPath, ResolvedListRequirements>,
    pub(crate) forbidden_string_list_globs:
        BTreeMap<JsonPath, ResolvedForbiddenGlobRequirements<JsonStringGlob>>,
    pub(crate) object_keys: BTreeMap<JsonPath, ResolvedItemRequirements<KeyedItem<()>>>,
}

impl ResolvedJsonFileRequirements {
    #[must_use]
    pub const fn scalar_values(&self) -> &ResolvedMap<JsonPath, ScalarAssertion<ConfigScalar>> {
        &self.scalar_values
    }

    #[must_use]
    pub const fn string_lists(&self) -> &BTreeMap<JsonPath, ResolvedListRequirements> {
        &self.string_lists
    }

    #[must_use]
    pub const fn forbidden_string_list_globs(
        &self,
    ) -> &BTreeMap<JsonPath, ResolvedForbiddenGlobRequirements<JsonStringGlob>> {
        &self.forbidden_string_list_globs
    }

    #[must_use]
    pub const fn object_keys(
        &self,
    ) -> &BTreeMap<JsonPath, ResolvedItemRequirements<KeyedItem<()>>> {
        &self.object_keys
    }
}

impl EngineRequirement for JsonFileRequirements {
    fn engine_id(&self) -> &'static str {
        crate::ENGINE_ID
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
