use std::collections::BTreeMap;

use aqc_file_engine_core::{ConflictEntry, Provenance, push_conflict, push_rendered_conflict};

use super::collect::{
    JsonPresencePolarity, JsonPresenceRequirement, JsonPresenceSource, JsonValueKindRequirement,
    KindInput, KindInputs, MergeInputs, ObjectInput, ObjectInputs, PresenceInput, PresenceInputs,
};
use crate::types::JsonPath;

type Contributor = (Provenance, String);
type Contributors = Vec<Contributor>;
type DescendantContributors = BTreeMap<String, Contributors>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum KindPairStatus {
    Compatible,
    ExplainedByPresence,
    KindConflict,
}

pub(super) fn report_structural_conflicts(
    inputs: &MergeInputs,
    conflicts: &mut Vec<ConflictEntry>,
) {
    report_presence_conflicts(&inputs.presence, conflicts);
    report_kind_conflicts(&inputs.kinds, conflicts);
    report_root_kind_conflicts(&inputs.kinds, conflicts);
    report_prefix_conflicts(&inputs.kinds, &inputs.presence, conflicts);
    report_object_descendant_conflicts(&inputs.object_keys, &inputs.presence, conflicts);
}

fn report_presence_conflicts(presence: &PresenceInputs, conflicts: &mut Vec<ConflictEntry>) {
    for (path, inputs) in presence {
        if !has_cross_source_presence_conflict(inputs) {
            continue;
        }
        push_rendered_conflict(
            path.finding_key(),
            "json-value-required-and-forbidden",
            inputs
                .iter()
                .filter(|input| participates_in_cross_source_conflict(input, inputs))
                .map(|(provenance, requirement, _)| {
                    let rendered = match requirement {
                        JsonPresenceRequirement::Required(message) => {
                            format!("required: {message}")
                        }
                        JsonPresenceRequirement::Forbidden(message) => {
                            format!("forbidden: {message}")
                        }
                    };
                    (provenance.clone(), rendered)
                })
                .collect(),
            conflicts,
        );
    }
}

fn has_cross_source_presence_conflict(inputs: &[PresenceInput]) -> bool {
    inputs.iter().any(|input| {
        inputs.iter().any(|other| {
            input.2 != other.2
                && matches!(input.1, JsonPresenceRequirement::Required(_))
                && matches!(other.1, JsonPresenceRequirement::Forbidden(_))
        })
    })
}

fn participates_in_cross_source_conflict(input: &PresenceInput, inputs: &[PresenceInput]) -> bool {
    inputs.iter().any(|other| {
        input.2 != other.2
            && matches!(
                (&input.1, &other.1),
                (
                    JsonPresenceRequirement::Required(_),
                    JsonPresenceRequirement::Forbidden(_)
                ) | (
                    JsonPresenceRequirement::Forbidden(_),
                    JsonPresenceRequirement::Required(_)
                )
            )
    })
}

fn report_kind_conflicts(kinds: &KindInputs, conflicts: &mut Vec<ConflictEntry>) {
    for (path, inputs) in kinds {
        let conflict_inputs = unexplained_kind_conflict_inputs(inputs);
        if !conflict_inputs.is_empty() {
            push_conflict(
                path.finding_key(),
                "json-value-kind-disagree",
                &conflict_inputs,
                render_kind,
                conflicts,
            );
        }
    }
}

fn unexplained_kind_conflict_inputs(kinds: &[KindInput]) -> Vec<KindInput> {
    let mut conflicts = Vec::new();
    let mut remaining = kinds;
    while let Some((left, rest)) = remaining.split_first() {
        let left_kind = &left.1;
        for right in rest {
            let right_kind = &right.1;
            if same_value_kind(left_kind, right_kind) {
                continue;
            }
            if classify_kind_pair(left_kind, right_kind) == KindPairStatus::KindConflict {
                push_unique_kind_input(&mut conflicts, left);
                push_unique_kind_input(&mut conflicts, right);
            }
        }
        remaining = rest;
    }
    conflicts
}

fn push_unique_kind_input(target: &mut Vec<KindInput>, input: &KindInput) {
    if !target.contains(input) {
        target.push(input.clone());
    }
}

const fn classify_kind_pair(
    left_kind: &JsonValueKindRequirement,
    right_kind: &JsonValueKindRequirement,
) -> KindPairStatus {
    match (kind_presence(left_kind), kind_presence(right_kind)) {
        (Some(JsonPresencePolarity::Required), Some(JsonPresencePolarity::Forbidden))
        | (Some(JsonPresencePolarity::Forbidden), Some(JsonPresencePolarity::Required)) => {
            KindPairStatus::ExplainedByPresence
        }
        (Some(JsonPresencePolarity::Required), _) | (_, Some(JsonPresencePolarity::Required)) => {
            KindPairStatus::KindConflict
        }
        (
            Some(JsonPresencePolarity::Forbidden) | None,
            Some(JsonPresencePolarity::Forbidden) | None,
        ) => KindPairStatus::Compatible,
    }
}

const fn kind_presence(kind: &JsonValueKindRequirement) -> Option<JsonPresencePolarity> {
    match kind {
        JsonValueKindRequirement::Scalar(polarity) => Some(*polarity),
        JsonValueKindRequirement::StringList(polarity)
        | JsonValueKindRequirement::Object(polarity) => *polarity,
    }
}

const fn same_value_kind(
    left: &JsonValueKindRequirement,
    right: &JsonValueKindRequirement,
) -> bool {
    matches!(
        (left, right),
        (
            JsonValueKindRequirement::Scalar(_),
            JsonValueKindRequirement::Scalar(_)
        ) | (
            JsonValueKindRequirement::StringList(_),
            JsonValueKindRequirement::StringList(_)
        ) | (
            JsonValueKindRequirement::Object(_),
            JsonValueKindRequirement::Object(_)
        )
    )
}

fn report_prefix_conflicts(
    kinds: &KindInputs,
    presence: &PresenceInputs,
    conflicts: &mut Vec<ConflictEntry>,
) {
    for (ancestor, ancestor_inputs) in kinds {
        let ancestor_is_leaf = ancestor_inputs.iter().any(|(_, kind)| {
            matches!(
                kind,
                JsonValueKindRequirement::Scalar(_) | JsonValueKindRequirement::StringList(_)
            )
        });
        if !ancestor_is_leaf {
            continue;
        }
        for (descendant, descendant_inputs) in presence {
            if ancestor == descendant
                || !path_is_prefix(ancestor, descendant)
                || !path_requires_presence(descendant, presence)
            {
                continue;
            }
            let mut contributors = ancestor_inputs
                .iter()
                .map(|(provenance, kind)| {
                    (provenance.clone(), format!("managed {}", render_kind(kind)))
                })
                .collect::<Vec<_>>();
            contributors.extend(required_contributors(descendant_inputs));
            push_rendered_conflict(
                ancestor.finding_key(),
                "json-leaf-has-required-descendant",
                contributors,
                conflicts,
            );
        }
    }
}

fn report_root_kind_conflicts(kinds: &KindInputs, conflicts: &mut Vec<ConflictEntry>) {
    let Some(inputs) = kinds.get(&JsonPath::root()) else {
        return;
    };
    let leaf_inputs = inputs
        .iter()
        .filter(|(_, kind)| {
            matches!(
                kind,
                JsonValueKindRequirement::Scalar(_) | JsonValueKindRequirement::StringList(_)
            )
        })
        .cloned()
        .collect::<Vec<_>>();
    if !leaf_inputs.is_empty() {
        push_conflict(
            "$",
            "json-root-must-be-object",
            &leaf_inputs,
            render_kind,
            conflicts,
        );
    }
}

fn report_object_descendant_conflicts(
    objects: &ObjectInputs,
    presence: &PresenceInputs,
    conflicts: &mut Vec<ConflictEntry>,
) {
    for (object_path, object_inputs) in objects {
        let object_components = object_path.components().collect::<Vec<_>>();
        let mut descendants = DescendantContributors::new();
        for (descendant_path, descendant_inputs) in presence {
            if !path_is_prefix(object_path, descendant_path) {
                continue;
            }
            let Some(child) = descendant_path.components().nth(object_components.len()) else {
                continue;
            };
            let contributors =
                descendant_inputs
                    .iter()
                    .filter_map(
                        |(provenance, requirement, source)| match (requirement, source) {
                            (
                                JsonPresenceRequirement::Required(_),
                                JsonPresenceSource::Object(owner),
                            ) if owner == object_path => None,
                            (JsonPresenceRequirement::Required(message), _) => {
                                Some((provenance.clone(), message.clone()))
                            }
                            (JsonPresenceRequirement::Forbidden(_), _) => None,
                        },
                    );
            descendants
                .entry(child.to_owned())
                .or_default()
                .extend(contributors);
        }
        for (child, descendant_inputs) in descendants {
            if descendant_inputs.is_empty() {
                continue;
            }
            let mut contributors = closure_contributors(&child, object_inputs);
            if contributors.is_empty() {
                continue;
            }
            contributors.extend(descendant_inputs);
            push_rendered_conflict(
                object_path.clone().child(child).finding_key(),
                "json-object-closure-excludes-managed-descendant",
                contributors,
                conflicts,
            );
        }
    }
}

fn path_requires_presence(path: &JsonPath, presence: &PresenceInputs) -> bool {
    presence.get(path).is_some_and(|inputs| {
        inputs
            .iter()
            .any(|(_, requirement, _)| matches!(requirement, JsonPresenceRequirement::Required(_)))
    })
}

fn required_contributors(inputs: &[PresenceInput]) -> Contributors {
    inputs
        .iter()
        .filter_map(|(provenance, requirement, _)| match requirement {
            JsonPresenceRequirement::Required(message) => {
                Some((provenance.clone(), message.clone()))
            }
            JsonPresenceRequirement::Forbidden(_) => None,
        })
        .collect()
}

fn closure_contributors(child: &str, object_inputs: &[ObjectInput]) -> Contributors {
    let mut contributors = Vec::new();
    for (provenance, requirement) in object_inputs {
        contributors.extend(
            requirement
                .forbidden
                .iter()
                .filter(|(item, _)| item.file_key == child)
                .map(|(_, message)| (provenance.clone(), message.clone())),
        );
        let Some((exact, message)) = &requirement.exact else {
            continue;
        };
        if !exact.iter().any(|item| item.file_key == child) {
            contributors.push((provenance.clone(), message.clone()));
        }
    }
    contributors
}

fn path_is_prefix(ancestor: &JsonPath, descendant: &JsonPath) -> bool {
    let ancestor = ancestor.components().collect::<Vec<_>>();
    let descendant = descendant.components().collect::<Vec<_>>();
    ancestor.len() < descendant.len()
        && ancestor
            .iter()
            .zip(descendant.iter())
            .all(|(left, right)| left == right)
}

fn render_kind(kind: &JsonValueKindRequirement) -> String {
    match kind {
        JsonValueKindRequirement::Scalar(_) => "scalar".to_owned(),
        JsonValueKindRequirement::StringList(_) => "string-list".to_owned(),
        JsonValueKindRequirement::Object(_) => "object".to_owned(),
    }
}
