//! Rust toolchain requirement merge logic.

use aqc_file_engine_core::{
    ConflictEntry, FileKeyRequirement, ListRequirements, Provenance, Resolve, ScalarAssertion,
    push_rendered_conflict, resolve_key_membership, resolve_list,
};

use super::{
    ResolvedRustToolchainTomlRequirements, RustToolchainChannel, RustToolchainPath,
    RustToolchainProfile, RustToolchainTomlRequirements,
};

type RustToolchainRequirementInput = Vec<(Provenance, RustToolchainTomlRequirements)>;

impl RustToolchainTomlRequirements {
    /// Merges all rust-toolchain TOML requirements into one resolved requirement set.
    ///
    /// # Errors
    ///
    /// Returns every conflict when the input requirements cannot be composed.
    #[expect(
        clippy::needless_pass_by_value,
        reason = "The shared merged_reconcile contract supplies an owned requirement vector."
    )]
    pub fn merge(
        reqs: RustToolchainRequirementInput,
    ) -> Result<ResolvedRustToolchainTomlRequirements, Vec<ConflictEntry>> {
        let mut conflicts = Vec::new();
        let channel =
            resolve_optional_scalar("toolchain.channel", &reqs, field_channel, &mut conflicts);
        let path = resolve_optional_scalar("toolchain.path", &reqs, field_path, &mut conflicts);
        let profile =
            resolve_optional_scalar("toolchain.profile", &reqs, field_profile, &mut conflicts);
        let components = resolve_list(
            "toolchain.components",
            reqs.iter()
                .map(|(prov, req)| (prov.clone(), normalize_list(req.components.clone())))
                .collect(),
            &mut conflicts,
        );
        let targets = resolve_list(
            "toolchain.targets",
            reqs.iter()
                .map(|(prov, req)| (prov.clone(), normalize_list(req.targets.clone())))
                .collect(),
            &mut conflicts,
        );
        let toolchain_keys = resolve_key_membership(
            "toolchain",
            reqs.iter()
                .map(|(provenance, requirement)| {
                    (provenance.clone(), requirement.toolchain_keys.clone())
                })
                .collect(),
            reqs.iter()
                .map(|(provenance, requirement)| {
                    (provenance.clone(), toolchain_key_constraints(requirement))
                })
                .collect(),
            &mut conflicts,
        );
        reject_empty_exact_toolchain(&toolchain_keys, &mut conflicts);

        let resolved = ResolvedRustToolchainTomlRequirements {
            channel,
            path,
            profile,
            components,
            targets,
            toolchain_keys,
        };

        if conflicts.is_empty() {
            Ok(resolved)
        } else {
            Err(conflicts)
        }
    }
}

fn reject_empty_exact_toolchain(
    requirement: &aqc_file_engine_core::ResolvedItemRequirements<
        aqc_file_engine_core::KeyedItem<()>,
    >,
    conflicts: &mut Vec<ConflictEntry>,
) {
    let Some(exact) = requirement
        .exact
        .as_ref()
        .filter(|exact| exact.identities.is_empty())
    else {
        return;
    };
    push_rendered_conflict(
        "toolchain",
        "empty-toolchain-table",
        exact
            .collected
            .iter()
            .map(|(provenance, _)| (provenance.clone(), "exact []".to_owned()))
            .collect(),
        conflicts,
    );
}

fn toolchain_key_constraints(
    requirement: &RustToolchainTomlRequirements,
) -> aqc_file_engine_core::ItemRequirements<aqc_file_engine_core::KeyedItem<()>> {
    let mut constraints = aqc_file_engine_core::ItemRequirements::default();
    if let Some(assertion) = &requirement.channel {
        assertion.constrain_file_key("channel", &mut constraints);
    }
    if let Some(assertion) = &requirement.path {
        assertion.constrain_file_key("path", &mut constraints);
    }
    if let Some(assertion) = &requirement.profile {
        assertion.constrain_file_key("profile", &mut constraints);
    }
    requirement
        .components
        .constrain_file_key("components", &mut constraints);
    requirement
        .targets
        .constrain_file_key("targets", &mut constraints);
    constraints
}

fn resolve_optional_scalar<T>(
    key: &str,
    reqs: &[(Provenance, RustToolchainTomlRequirements)],
    get: impl Fn(&RustToolchainTomlRequirements) -> Option<&ScalarAssertion<T>>,
    conflicts: &mut Vec<ConflictEntry>,
) -> Option<aqc_file_engine_core::ResolvedRequirement<ScalarAssertion<T>, ScalarAssertion<T>>>
where
    T: aqc_file_engine_core::ScalarValue,
{
    let items = reqs
        .iter()
        .filter_map(|(prov, req)| get(req).cloned().map(|assertion| (prov.clone(), assertion)))
        .collect::<Vec<_>>();
    if items.is_empty() {
        None
    } else {
        ScalarAssertion::<T>::resolve(key, items, conflicts)
    }
}

fn field_channel(
    req: &RustToolchainTomlRequirements,
) -> Option<&ScalarAssertion<RustToolchainChannel>> {
    req.channel.as_ref()
}

fn field_path(req: &RustToolchainTomlRequirements) -> Option<&ScalarAssertion<RustToolchainPath>> {
    req.path.as_ref()
}

fn field_profile(
    req: &RustToolchainTomlRequirements,
) -> Option<&ScalarAssertion<RustToolchainProfile>> {
    req.profile.as_ref()
}

fn normalize_list(mut list: ListRequirements) -> ListRequirements {
    if let Some((values, message)) = list.exact {
        let mut sorted = values;
        sorted.sort();
        sorted.dedup();
        list.exact = Some((sorted, message));
    }
    list
}
