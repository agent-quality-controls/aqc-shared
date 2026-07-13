//! Rust toolchain requirement merge logic.

use aqc_file_engine_core::{
    ConflictEntry, ListRequirements, Provenance, Resolve, ScalarAssertion, resolve_list,
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
        let exact_settings = reqs
            .into_iter()
            .filter_map(|(prov, req)| req.exact_settings.map(|message| (prov, message)))
            .collect();

        let resolved = ResolvedRustToolchainTomlRequirements {
            channel,
            path,
            profile,
            components,
            targets,
            exact_settings,
        };

        if conflicts.is_empty() {
            Ok(resolved)
        } else {
            Err(conflicts)
        }
    }
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
