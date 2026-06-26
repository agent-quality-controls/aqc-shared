//! `[workspace].<key>` assertions (resolver, members, exclude, default-members).

#![cfg_attr(
    not(test),
    expect(
        clippy::missing_docs_in_private_items,
        reason = "Private workspace-field composition helpers are internal requirement steps."
    )
)]
#![expect(
    clippy::option_option,
    reason = "Workspace-field composition uses product wrappers and three-state resolution."
)]

use aqc_file_engine_core::{
    ConfigScalar, ConflictEntry, ListRequirements, OnEmpty, OnEmptyClass, Provenance, Resolve,
    ResolvedListRequirements, ResolvedRequirement, ScalarAssertion,
    push_conflict as push_core_conflict, render_list_requirement, render_scalar_assertion,
    resolve_list,
};

/// One provenance-tagged workspace assertion.
type WorkspaceFieldInput = (Provenance, WorkspaceFieldAssertion);
/// Borrowed workspace assertions for one key.
type WorkspaceFieldInputSlice<'a> = &'a [WorkspaceFieldInput];

/// What must hold about a direct `[workspace]` key.
#[derive(Debug, Clone)]
pub enum WorkspaceFieldAssertion {
    Scalar(ScalarAssertion<ConfigScalar>),
    List(ListRequirements),
}

/// Resolved workspace field assertion.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResolvedWorkspaceFieldAssertion {
    Scalar(ScalarAssertion<ConfigScalar>),
    List(ResolvedListRequirements),
}

impl PartialEq for WorkspaceFieldAssertion {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Scalar(a), Self::Scalar(b)) => a == b,
            (Self::List(a), Self::List(b)) => a == b,
            (Self::Scalar(_), Self::List(_)) | (Self::List(_), Self::Scalar(_)) => false,
        }
    }
}

impl Resolve for WorkspaceFieldAssertion {
    type Merged = ResolvedWorkspaceFieldAssertion;

    fn resolve(
        key: &str,
        items: Vec<(Provenance, Self)>,
        conflicts: &mut Vec<ConflictEntry>,
    ) -> Option<ResolvedRequirement<Self::Merged, Self>> {
        if !items
            .iter()
            .all(|(_, assertion)| workspace_field_assertion_is_legal(key, assertion))
        {
            push_unsupported_conflict(key, &items, conflicts);
            return None;
        }
        if let Some(resolved) = resolve_list_product(key, &items, conflicts) {
            return resolved.map(|merged| ResolvedRequirement {
                merged,
                collected: items,
            });
        }
        if !items
            .iter()
            .all(|(_, assertion)| matches!(assertion, Self::Scalar(_)))
        {
            push_scalar_disagree_conflict(key, &items, conflicts);
            return None;
        }
        let scalar_items = items
            .iter()
            .filter_map(|(prov, assertion)| match assertion {
                Self::Scalar(assertion) => Some((prov.clone(), assertion.clone())),
                Self::List(_) => None,
            })
            .collect();
        let resolved = ScalarAssertion::<ConfigScalar>::resolve(key, scalar_items, conflicts)?;
        Some(ResolvedRequirement {
            merged: ResolvedWorkspaceFieldAssertion::Scalar(resolved.merged),
            collected: items,
        })
    }
}

impl OnEmptyClass for WorkspaceFieldAssertion {
    fn on_empty(&self) -> OnEmpty {
        match self {
            Self::Scalar(assertion) => assertion.on_empty(),
            Self::List(_) => OnEmpty::Writes,
        }
    }
}

impl OnEmptyClass for ResolvedWorkspaceFieldAssertion {
    fn on_empty(&self) -> OnEmpty {
        match self {
            Self::Scalar(assertion) => assertion.on_empty(),
            Self::List(_) => OnEmpty::Writes,
        }
    }
}

fn resolve_list_product(
    key: &str,
    items: WorkspaceFieldInputSlice<'_>,
    conflicts: &mut Vec<ConflictEntry>,
) -> Option<Option<ResolvedWorkspaceFieldAssertion>> {
    if !items
        .iter()
        .any(|(_, assertion)| matches!(assertion, WorkspaceFieldAssertion::List(_)))
    {
        return None;
    }
    if !items.iter().all(|(_, assertion)| {
        matches!(assertion, WorkspaceFieldAssertion::List(_))
            || matches!(
                assertion,
                WorkspaceFieldAssertion::Scalar(ScalarAssertion::Present(_))
            )
    }) {
        push_scalar_disagree_conflict(key, items, conflicts);
        return Some(None);
    }
    let list_items = items
        .iter()
        .filter_map(|(prov, assertion)| match assertion {
            WorkspaceFieldAssertion::List(list) => Some((prov.clone(), list.clone())),
            WorkspaceFieldAssertion::Scalar(_) => None,
        })
        .collect();
    Some(Some(ResolvedWorkspaceFieldAssertion::List(resolve_list(
        key, list_items, conflicts,
    ))))
}

fn push_scalar_disagree_conflict(
    key: &str,
    items: WorkspaceFieldInputSlice<'_>,
    conflicts: &mut Vec<ConflictEntry>,
) {
    push_core_conflict(
        key,
        "scalar-disagree",
        items,
        render_workspace_field_assertion,
        conflicts,
    );
}

fn workspace_field_assertion_is_legal(key: &str, assertion: &WorkspaceFieldAssertion) -> bool {
    let field = key.rsplit('.').next().unwrap_or(key);
    match field {
        "members" | "exclude" | "default-members" => matches!(
            assertion,
            WorkspaceFieldAssertion::List(_)
                | WorkspaceFieldAssertion::Scalar(
                    ScalarAssertion::Present(_) | ScalarAssertion::Absent(_)
                )
        ),
        "resolver" => match assertion {
            WorkspaceFieldAssertion::Scalar(
                ScalarAssertion::Equals(ConfigScalar::Str(_), _)
                | ScalarAssertion::Present(_)
                | ScalarAssertion::Absent(_),
            ) => true,
            WorkspaceFieldAssertion::Scalar(ScalarAssertion::OneOf(values, _)) => values
                .iter()
                .all(|value| matches!(value, ConfigScalar::Str(_))),
            WorkspaceFieldAssertion::List(_) | WorkspaceFieldAssertion::Scalar(_) => false,
        },
        _ => matches!(assertion, WorkspaceFieldAssertion::Scalar(_)),
    }
}

fn push_unsupported_conflict(
    key: &str,
    items: WorkspaceFieldInputSlice<'_>,
    conflicts: &mut Vec<ConflictEntry>,
) {
    push_core_conflict(
        key,
        "scalar-operation-unsupported",
        items,
        render_workspace_field_assertion,
        conflicts,
    );
}

fn render_workspace_field_assertion(assertion: &WorkspaceFieldAssertion) -> String {
    match assertion {
        WorkspaceFieldAssertion::Scalar(assertion) => render_scalar_assertion(assertion),
        WorkspaceFieldAssertion::List(requirements) => render_list_requirement(requirements),
    }
}
