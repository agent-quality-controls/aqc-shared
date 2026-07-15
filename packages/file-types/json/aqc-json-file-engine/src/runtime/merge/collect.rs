use std::collections::BTreeMap;

use aqc_file_engine_core::{
    ConfigScalar, ForbiddenGlobRequirements, ItemRequirements, KeyedItem, ListRequirements,
    Provenance, ScalarAssertion,
};

use crate::types::{JsonFileRequirements, JsonPath, JsonStringGlob};

pub(super) type KindInput = (Provenance, JsonValueKindRequirement);
pub(super) type KindInputs = BTreeMap<JsonPath, Vec<KindInput>>;
pub(super) type PresenceInput = (Provenance, JsonPresenceRequirement, JsonPresenceSource);
pub(super) type PresenceInputs = BTreeMap<JsonPath, Vec<PresenceInput>>;
pub(super) type ObjectRequirement = ItemRequirements<KeyedItem<()>>;
pub(super) type ObjectInput = (Provenance, ObjectRequirement);
pub(super) type ObjectInputs = BTreeMap<JsonPath, Vec<ObjectInput>>;
pub(super) type RequirementContributions = Vec<(Provenance, JsonFileRequirements)>;

type ScalarAssertions = BTreeMap<JsonPath, ScalarAssertion<ConfigScalar>>;
type ScalarInputs = Vec<(Provenance, ScalarAssertions)>;
type ListInputs = BTreeMap<JsonPath, Vec<(Provenance, ListRequirements)>>;
type GlobRequirement = ForbiddenGlobRequirements<JsonStringGlob>;
type GlobInputs = BTreeMap<JsonPath, Vec<(Provenance, GlobRequirement)>>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum JsonValueKindRequirement {
    Scalar(JsonPresencePolarity),
    StringList(Option<JsonPresencePolarity>),
    Object(Option<JsonPresencePolarity>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum JsonPresencePolarity {
    Required,
    Forbidden,
}

#[derive(Debug, Clone)]
pub(super) enum JsonPresenceRequirement {
    Required(String),
    Forbidden(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum JsonPresenceSource {
    Scalar,
    StringList,
    Object(JsonPath),
}

pub(super) struct MergeInputs {
    pub(super) scalar_values: ScalarInputs,
    pub(super) string_lists: ListInputs,
    pub(super) forbidden_string_list_globs: GlobInputs,
    pub(super) object_keys: ObjectInputs,
    pub(super) kinds: KindInputs,
    pub(super) presence: PresenceInputs,
}

impl MergeInputs {
    pub(super) fn collect(requirements: RequirementContributions) -> Self {
        let mut inputs = Self {
            scalar_values: Vec::new(),
            string_lists: BTreeMap::new(),
            forbidden_string_list_globs: BTreeMap::new(),
            object_keys: BTreeMap::new(),
            kinds: BTreeMap::new(),
            presence: BTreeMap::new(),
        };
        for (provenance, requirement) in requirements {
            let JsonFileRequirements {
                scalar_values,
                string_lists,
                forbidden_string_list_globs,
                object_keys,
            } = requirement;
            collect_scalar_requirements(
                &provenance,
                &scalar_values,
                &mut inputs.kinds,
                &mut inputs.presence,
            );
            inputs
                .scalar_values
                .push((provenance.clone(), scalar_values));
            collect_list_requirements(
                &provenance,
                string_lists,
                &mut inputs.string_lists,
                &mut inputs.kinds,
                &mut inputs.presence,
            );
            collect_glob_requirements(
                &provenance,
                forbidden_string_list_globs,
                &mut inputs.forbidden_string_list_globs,
                &mut inputs.kinds,
            );
            collect_object_requirements(
                &provenance,
                object_keys,
                &mut inputs.object_keys,
                &mut inputs.kinds,
                &mut inputs.presence,
            );
        }
        inputs
    }
}

fn collect_scalar_requirements(
    provenance: &Provenance,
    scalar_values: &ScalarAssertions,
    kinds: &mut KindInputs,
    presence: &mut PresenceInputs,
) {
    for (path, assertion) in scalar_values {
        let presence_requirement = match assertion {
            ScalarAssertion::Absent(message) => JsonPresenceRequirement::Forbidden(message.clone()),
            ScalarAssertion::Equals(_, message)
            | ScalarAssertion::AtLeast(_, message)
            | ScalarAssertion::AtMost(_, message)
            | ScalarAssertion::Range(_, _, message)
            | ScalarAssertion::OneOf(_, message)
            | ScalarAssertion::Present(message) => {
                JsonPresenceRequirement::Required(message.clone())
            }
        };
        let polarity = presence_requirement.polarity();
        kinds.entry(path.clone()).or_default().push((
            provenance.clone(),
            JsonValueKindRequirement::Scalar(polarity),
        ));
        presence.entry(path.clone()).or_default().push((
            provenance.clone(),
            presence_requirement,
            JsonPresenceSource::Scalar,
        ));
    }
}

fn collect_list_requirements(
    provenance: &Provenance,
    requirements: BTreeMap<JsonPath, ListRequirements>,
    inputs: &mut ListInputs,
    kinds: &mut KindInputs,
    presence: &mut PresenceInputs,
) {
    for (path, list) in requirements.into_iter().filter(|(_, list)| {
        !list.contains.is_empty() || !list.excludes.is_empty() || list.exact.is_some()
    }) {
        collect_list_requirement(provenance, &path, &list, kinds, presence);
        inputs
            .entry(path)
            .or_default()
            .push((provenance.clone(), list));
    }
}

fn collect_list_requirement(
    provenance: &Provenance,
    path: &JsonPath,
    list: &ListRequirements,
    kinds: &mut KindInputs,
    presence: &mut PresenceInputs,
) {
    for message in list
        .contains
        .values()
        .chain(list.exact.iter().map(|(_, message)| message))
    {
        presence.entry(path.clone()).or_default().push((
            provenance.clone(),
            JsonPresenceRequirement::Required(message.clone()),
            JsonPresenceSource::StringList,
        ));
    }
    kinds.entry(path.clone()).or_default().push((
        provenance.clone(),
        JsonValueKindRequirement::StringList(
            (!list.contains.is_empty() || list.exact.is_some())
                .then_some(JsonPresencePolarity::Required),
        ),
    ));
}

fn collect_glob_requirements(
    provenance: &Provenance,
    requirements: BTreeMap<JsonPath, GlobRequirement>,
    inputs: &mut GlobInputs,
    kinds: &mut KindInputs,
) {
    for (path, globs) in requirements
        .into_iter()
        .filter(|(_, globs)| !globs.globs.is_empty())
    {
        kinds.entry(path.clone()).or_default().push((
            provenance.clone(),
            JsonValueKindRequirement::StringList(None),
        ));
        inputs
            .entry(path)
            .or_default()
            .push((provenance.clone(), globs));
    }
}

fn collect_object_requirements(
    provenance: &Provenance,
    requirements: BTreeMap<JsonPath, ObjectRequirement>,
    inputs: &mut ObjectInputs,
    kinds: &mut KindInputs,
    presence: &mut PresenceInputs,
) {
    for (path, keys) in requirements.into_iter().filter(|(_, keys)| {
        !keys.required.is_empty() || !keys.forbidden.is_empty() || keys.exact.is_some()
    }) {
        collect_object_requirement(provenance, &path, &keys, kinds, presence);
        inputs
            .entry(path)
            .or_default()
            .push((provenance.clone(), keys));
    }
}

fn collect_object_requirement(
    provenance: &Provenance,
    path: &JsonPath,
    keys: &ObjectRequirement,
    kinds: &mut KindInputs,
    presence: &mut PresenceInputs,
) {
    for (item, message) in &keys.required {
        push_presence(
            presence,
            path.clone().child(item.file_key.clone()),
            provenance,
            JsonPresenceRequirement::Required(message.clone()),
            JsonPresenceSource::Object(path.clone()),
        );
    }
    for (item, message) in &keys.forbidden {
        push_presence(
            presence,
            path.clone().child(item.file_key.clone()),
            provenance,
            JsonPresenceRequirement::Forbidden(message.clone()),
            JsonPresenceSource::Object(path.clone()),
        );
    }
    if let Some((exact, message)) = &keys.exact {
        push_presence(
            presence,
            path.clone(),
            provenance,
            JsonPresenceRequirement::Required(message.clone()),
            JsonPresenceSource::Object(path.clone()),
        );
        for item in exact {
            push_presence(
                presence,
                path.clone().child(item.file_key.clone()),
                provenance,
                JsonPresenceRequirement::Required(message.clone()),
                JsonPresenceSource::Object(path.clone()),
            );
        }
    } else {
        for (_, message) in &keys.required {
            push_presence(
                presence,
                path.clone(),
                provenance,
                JsonPresenceRequirement::Required(message.clone()),
                JsonPresenceSource::Object(path.clone()),
            );
        }
    }
    kinds.entry(path.clone()).or_default().push((
        provenance.clone(),
        JsonValueKindRequirement::Object(
            (!keys.required.is_empty() || keys.exact.is_some())
                .then_some(JsonPresencePolarity::Required),
        ),
    ));
}

fn push_presence(
    presence: &mut PresenceInputs,
    path: JsonPath,
    provenance: &Provenance,
    requirement: JsonPresenceRequirement,
    source: JsonPresenceSource,
) {
    presence
        .entry(path)
        .or_default()
        .push((provenance.clone(), requirement, source));
}

impl JsonPresenceRequirement {
    const fn polarity(&self) -> JsonPresencePolarity {
        match self {
            Self::Required(_) => JsonPresencePolarity::Required,
            Self::Forbidden(_) => JsonPresencePolarity::Forbidden,
        }
    }
}
