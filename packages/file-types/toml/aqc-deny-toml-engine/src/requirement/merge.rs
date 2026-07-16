//! Deny TOML requirement merge logic.

use std::collections::BTreeSet;

use aqc_file_engine_core::{
    ConflictEntry, FileKeyRequirement, ItemRequirements, KeyedItem, Provenance, ScalarAssertion,
    resolve_key_membership,
};

use super::merge_helpers::{item, list, report_feature_overlaps, scalar};
use super::{DenyTable, DenyTomlRequirements, ResolvedDenyTomlRequirements};

type DenyRequirementInput = Vec<(Provenance, DenyTomlRequirements)>;

impl DenyTomlRequirements {
    /// Merges all deny TOML requirements into one resolved requirement set.
    ///
    /// # Errors
    ///
    /// Returns every conflict when the input requirements cannot be composed.
    #[expect(
        clippy::too_many_lines,
        reason = "The merge surface intentionally mirrors every managed deny.toml field."
    )]
    #[expect(
        clippy::needless_pass_by_value,
        reason = "The shared merged_reconcile contract supplies an owned requirement vector."
    )]
    pub fn merge(
        reqs: DenyRequirementInput,
    ) -> Result<ResolvedDenyTomlRequirements, Vec<ConflictEntry>> {
        let mut conflicts = Vec::new();
        let constrained_key_requirements = reqs
            .iter()
            .map(|(provenance, requirement)| {
                (
                    provenance.clone(),
                    with_table_key_constraints(requirement.clone()),
                )
            })
            .collect::<Vec<_>>();
        let _ = ScalarAssertion::<bool>::Present(String::new()).operation();

        let graph_targets = item(
            "graph.targets",
            &reqs,
            |req| req.graph_targets.clone(),
            &mut conflicts,
        );
        let graph_exclude = list(
            "graph.exclude",
            &reqs,
            |req| req.graph_exclude.clone(),
            &mut conflicts,
        );
        let graph_exclude_dev = scalar(
            "graph.exclude-dev",
            &reqs,
            |req| req.graph_exclude_dev.clone(),
            &mut conflicts,
        );
        let graph_exclude_unpublished = scalar(
            "graph.exclude-unpublished",
            &reqs,
            |req| req.graph_exclude_unpublished.clone(),
            &mut conflicts,
        );
        let graph_all_features = scalar(
            "graph.all-features",
            &reqs,
            |req| req.graph_all_features.clone(),
            &mut conflicts,
        );
        let graph_no_default_features = scalar(
            "graph.no-default-features",
            &reqs,
            |req| req.graph_no_default_features.clone(),
            &mut conflicts,
        );
        let graph_features = list(
            "graph.features",
            &reqs,
            |req| req.graph_features.clone(),
            &mut conflicts,
        );
        let output_feature_depth = scalar(
            "output.feature-depth",
            &reqs,
            |req| req.output_feature_depth.clone(),
            &mut conflicts,
        );
        let advisories_version = scalar(
            "advisories.version",
            &reqs,
            |req| req.advisories_version.clone(),
            &mut conflicts,
        );
        let advisories_db_path = scalar(
            "advisories.db-path",
            &reqs,
            |req| req.advisories_db_path.clone(),
            &mut conflicts,
        );
        let advisories_db_urls = list(
            "advisories.db-urls",
            &reqs,
            |req| req.advisories_db_urls.clone(),
            &mut conflicts,
        );
        let advisories_yanked = scalar(
            "advisories.yanked",
            &reqs,
            |req| req.advisories_yanked.clone(),
            &mut conflicts,
        );
        let advisories_disable_yank_checking = scalar(
            "advisories.disable-yank-checking",
            &reqs,
            |req| req.advisories_disable_yank_checking.clone(),
            &mut conflicts,
        );
        let advisories_ignore = item(
            "advisories.ignore",
            &reqs,
            |req| req.advisories_ignore.clone(),
            &mut conflicts,
        );
        let advisories_unmaintained = scalar(
            "advisories.unmaintained",
            &reqs,
            |req| req.advisories_unmaintained.clone(),
            &mut conflicts,
        );
        let advisories_unsound = scalar(
            "advisories.unsound",
            &reqs,
            |req| req.advisories_unsound.clone(),
            &mut conflicts,
        );
        let advisories_maximum_db_staleness = scalar(
            "advisories.maximum-db-staleness",
            &reqs,
            |req| req.advisories_maximum_db_staleness.clone(),
            &mut conflicts,
        );
        let advisories_git_fetch_with_cli = scalar(
            "advisories.git-fetch-with-cli",
            &reqs,
            |req| req.advisories_git_fetch_with_cli.clone(),
            &mut conflicts,
        );
        let advisories_unused_ignored_advisory = scalar(
            "advisories.unused-ignored-advisory",
            &reqs,
            |req| req.advisories_unused_ignored_advisory.clone(),
            &mut conflicts,
        );
        let licenses_version = scalar(
            "licenses.version",
            &reqs,
            |req| req.licenses_version.clone(),
            &mut conflicts,
        );
        let licenses_include_dev = scalar(
            "licenses.include-dev",
            &reqs,
            |req| req.licenses_include_dev.clone(),
            &mut conflicts,
        );
        let licenses_include_build = scalar(
            "licenses.include-build",
            &reqs,
            |req| req.licenses_include_build.clone(),
            &mut conflicts,
        );
        let licenses_allow = list(
            "licenses.allow",
            &reqs,
            |req| req.licenses_allow.clone(),
            &mut conflicts,
        );
        let licenses_exceptions = item(
            "licenses.exceptions",
            &reqs,
            |req| req.licenses_exceptions.clone(),
            &mut conflicts,
        );
        let licenses_confidence_threshold = scalar(
            "licenses.confidence-threshold",
            &reqs,
            |req| req.licenses_confidence_threshold.clone(),
            &mut conflicts,
        );
        let licenses_clarify = item(
            "licenses.clarify",
            &reqs,
            |req| req.licenses_clarify.clone(),
            &mut conflicts,
        );
        let licenses_private_ignore = scalar(
            "licenses.private.ignore",
            &reqs,
            |req| req.licenses_private_ignore.clone(),
            &mut conflicts,
        );
        let licenses_private_registries = list(
            "licenses.private.registries",
            &reqs,
            |req| req.licenses_private_registries.clone(),
            &mut conflicts,
        );
        let licenses_private_ignore_sources = list(
            "licenses.private.ignore-sources",
            &reqs,
            |req| req.licenses_private_ignore_sources.clone(),
            &mut conflicts,
        );
        let licenses_unused_allowed_license = scalar(
            "licenses.unused-allowed-license",
            &reqs,
            |req| req.licenses_unused_allowed_license.clone(),
            &mut conflicts,
        );
        let licenses_unused_license_exception = scalar(
            "licenses.unused-license-exception",
            &reqs,
            |req| req.licenses_unused_license_exception.clone(),
            &mut conflicts,
        );
        let bans_multiple_versions = scalar(
            "bans.multiple-versions",
            &reqs,
            |req| req.bans_multiple_versions.clone(),
            &mut conflicts,
        );
        let bans_multiple_versions_include_dev = scalar(
            "bans.multiple-versions-include-dev",
            &reqs,
            |req| req.bans_multiple_versions_include_dev.clone(),
            &mut conflicts,
        );
        let bans_wildcards = scalar(
            "bans.wildcards",
            &reqs,
            |req| req.bans_wildcards.clone(),
            &mut conflicts,
        );
        let bans_allow_wildcard_paths = scalar(
            "bans.allow-wildcard-paths",
            &reqs,
            |req| req.bans_allow_wildcard_paths.clone(),
            &mut conflicts,
        );
        let bans_highlight = scalar(
            "bans.highlight",
            &reqs,
            |req| req.bans_highlight.clone(),
            &mut conflicts,
        );
        let bans_workspace_default_features = scalar(
            "bans.workspace-default-features",
            &reqs,
            |req| req.bans_workspace_default_features.clone(),
            &mut conflicts,
        );
        let bans_external_default_features = scalar(
            "bans.external-default-features",
            &reqs,
            |req| req.bans_external_default_features.clone(),
            &mut conflicts,
        );
        let bans_allow = item(
            "bans.allow",
            &reqs,
            |req| req.bans_allow.clone(),
            &mut conflicts,
        );
        let bans_allow_workspace = scalar(
            "bans.allow-workspace",
            &reqs,
            |req| req.bans_allow_workspace.clone(),
            &mut conflicts,
        );
        let bans_deny = item(
            "bans.deny",
            &reqs,
            |req| req.bans_deny.clone(),
            &mut conflicts,
        );
        let bans_features = item(
            "bans.features",
            &reqs,
            |req| req.bans_features.clone(),
            &mut conflicts,
        );
        report_feature_overlaps(&bans_features, &mut conflicts);
        let bans_skip = item(
            "bans.skip",
            &reqs,
            |req| req.bans_skip.clone(),
            &mut conflicts,
        );
        let bans_skip_tree = item(
            "bans.skip-tree",
            &reqs,
            |req| req.bans_skip_tree.clone(),
            &mut conflicts,
        );
        let bans_workspace_dependencies_duplicates = scalar(
            "bans.workspace-dependencies.duplicates",
            &reqs,
            |req| req.bans_workspace_dependencies_duplicates.clone(),
            &mut conflicts,
        );
        let bans_workspace_dependencies_include_path_dependencies = scalar(
            "bans.workspace-dependencies.include-path-dependencies",
            &reqs,
            |req| {
                req.bans_workspace_dependencies_include_path_dependencies
                    .clone()
            },
            &mut conflicts,
        );
        let bans_workspace_dependencies_unused = scalar(
            "bans.workspace-dependencies.unused",
            &reqs,
            |req| req.bans_workspace_dependencies_unused.clone(),
            &mut conflicts,
        );
        let bans_build_executables = scalar(
            "bans.build.executables",
            &reqs,
            |req| req.bans_build_executables.clone(),
            &mut conflicts,
        );
        let bans_build_interpreted = scalar(
            "bans.build.interpreted",
            &reqs,
            |req| req.bans_build_interpreted.clone(),
            &mut conflicts,
        );
        let bans_build_script_extensions = list(
            "bans.build.script-extensions",
            &reqs,
            |req| req.bans_build_script_extensions.clone(),
            &mut conflicts,
        );
        let bans_build_enable_builtin_globs = scalar(
            "bans.build.enable-builtin-globs",
            &reqs,
            |req| req.bans_build_enable_builtin_globs.clone(),
            &mut conflicts,
        );
        let bans_build_globs = item(
            "bans.build.globs",
            &reqs,
            |req| req.bans_build_globs.clone(),
            &mut conflicts,
        );
        let bans_build_include_dependencies = scalar(
            "bans.build.include-dependencies",
            &reqs,
            |req| req.bans_build_include_dependencies.clone(),
            &mut conflicts,
        );
        let bans_build_include_workspace = scalar(
            "bans.build.include-workspace",
            &reqs,
            |req| req.bans_build_include_workspace.clone(),
            &mut conflicts,
        );
        let bans_build_include_archives = scalar(
            "bans.build.include-archives",
            &reqs,
            |req| req.bans_build_include_archives.clone(),
            &mut conflicts,
        );
        let sources_unknown_registry = scalar(
            "sources.unknown-registry",
            &reqs,
            |req| req.sources_unknown_registry.clone(),
            &mut conflicts,
        );
        let sources_unknown_git = scalar(
            "sources.unknown-git",
            &reqs,
            |req| req.sources_unknown_git.clone(),
            &mut conflicts,
        );
        let sources_required_git_spec = scalar(
            "sources.required-git-spec",
            &reqs,
            |req| req.sources_required_git_spec.clone(),
            &mut conflicts,
        );
        let sources_allow_git = list(
            "sources.allow-git",
            &reqs,
            |req| req.sources_allow_git.clone(),
            &mut conflicts,
        );
        let sources_private = list(
            "sources.private",
            &reqs,
            |req| req.sources_private.clone(),
            &mut conflicts,
        );
        let sources_allow_registry = list(
            "sources.allow-registry",
            &reqs,
            |req| req.sources_allow_registry.clone(),
            &mut conflicts,
        );
        let sources_allow_org_github = list(
            "sources.allow-org.github",
            &reqs,
            |req| req.sources_allow_org_github.clone(),
            &mut conflicts,
        );
        let sources_allow_org_gitlab = list(
            "sources.allow-org.gitlab",
            &reqs,
            |req| req.sources_allow_org_gitlab.clone(),
            &mut conflicts,
        );
        let sources_allow_org_bitbucket = list(
            "sources.allow-org.bitbucket",
            &reqs,
            |req| req.sources_allow_org_bitbucket.clone(),
            &mut conflicts,
        );
        let sources_unused_allowed_source = scalar(
            "sources.unused-allowed-source",
            &reqs,
            |req| req.sources_unused_allowed_source.clone(),
            &mut conflicts,
        );
        let table_keys = reqs
            .iter()
            .flat_map(|(_, requirement)| requirement.table_keys.keys().copied())
            .chain(
                constrained_key_requirements
                    .iter()
                    .flat_map(|(_, requirement)| requirement.table_keys.keys().copied()),
            )
            .collect::<BTreeSet<_>>()
            .into_iter()
            .map(|table| {
                let resolved = resolve_key_membership(
                    table.finding_key(),
                    reqs.iter()
                        .filter_map(|(provenance, requirement)| {
                            requirement
                                .table_keys
                                .get(&table)
                                .cloned()
                                .map(|keys| (provenance.clone(), keys))
                        })
                        .collect(),
                    constrained_key_requirements
                        .iter()
                        .filter_map(|(provenance, requirement)| {
                            requirement
                                .table_keys
                                .get(&table)
                                .cloned()
                                .map(|keys| (provenance.clone(), keys))
                        })
                        .collect(),
                    &mut conflicts,
                );
                (table, resolved)
            })
            .collect();

        let resolved = ResolvedDenyTomlRequirements {
            graph_targets,
            graph_exclude,
            graph_exclude_dev,
            graph_exclude_unpublished,
            graph_all_features,
            graph_no_default_features,
            graph_features,
            output_feature_depth,
            advisories_version,
            advisories_db_path,
            advisories_db_urls,
            advisories_yanked,
            advisories_disable_yank_checking,
            advisories_ignore,
            advisories_unmaintained,
            advisories_unsound,
            advisories_maximum_db_staleness,
            advisories_git_fetch_with_cli,
            advisories_unused_ignored_advisory,
            licenses_version,
            licenses_include_dev,
            licenses_include_build,
            licenses_allow,
            licenses_exceptions,
            licenses_confidence_threshold,
            licenses_clarify,
            licenses_private_ignore,
            licenses_private_registries,
            licenses_private_ignore_sources,
            licenses_unused_allowed_license,
            licenses_unused_license_exception,
            bans_multiple_versions,
            bans_multiple_versions_include_dev,
            bans_wildcards,
            bans_allow_wildcard_paths,
            bans_highlight,
            bans_workspace_default_features,
            bans_external_default_features,
            bans_allow,
            bans_allow_workspace,
            bans_deny,
            bans_features,
            bans_skip,
            bans_skip_tree,
            bans_workspace_dependencies_duplicates,
            bans_workspace_dependencies_include_path_dependencies,
            bans_workspace_dependencies_unused,
            bans_build_executables,
            bans_build_interpreted,
            bans_build_script_extensions,
            bans_build_enable_builtin_globs,
            bans_build_globs,
            bans_build_include_dependencies,
            bans_build_include_workspace,
            bans_build_include_archives,
            sources_unknown_registry,
            sources_unknown_git,
            sources_required_git_spec,
            sources_allow_git,
            sources_private,
            sources_allow_registry,
            sources_allow_org_github,
            sources_allow_org_gitlab,
            sources_allow_org_bitbucket,
            sources_unused_allowed_source,
            table_keys,
        };

        if conflicts.is_empty() {
            Ok(resolved)
        } else {
            Err(conflicts)
        }
    }
}

#[expect(
    clippy::too_many_lines,
    reason = "This exhaustive field-to-file-key inventory is kept together so new Deny fields cannot be hidden across partial helpers."
)]
fn with_table_key_constraints(source: DenyTomlRequirements) -> DenyTomlRequirements {
    let mut requirement = DenyTomlRequirements::default();
    macro_rules! field {
        ($table:ident, $key:literal, $field:ident) => {{
            let value = source.$field.clone();
            constrain_table_field(&mut requirement, DenyTable::$table, $key, &value);
        }};
    }

    field!(Graph, "targets", graph_targets);
    field!(Graph, "exclude", graph_exclude);
    field!(Graph, "exclude-dev", graph_exclude_dev);
    field!(Graph, "exclude-unpublished", graph_exclude_unpublished);
    field!(Graph, "all-features", graph_all_features);
    field!(Graph, "no-default-features", graph_no_default_features);
    field!(Graph, "features", graph_features);
    field!(Output, "feature-depth", output_feature_depth);
    field!(Advisories, "version", advisories_version);
    field!(Advisories, "db-path", advisories_db_path);
    field!(Advisories, "db-urls", advisories_db_urls);
    field!(Advisories, "yanked", advisories_yanked);
    field!(
        Advisories,
        "disable-yank-checking",
        advisories_disable_yank_checking
    );
    field!(Advisories, "ignore", advisories_ignore);
    field!(Advisories, "unmaintained", advisories_unmaintained);
    field!(Advisories, "unsound", advisories_unsound);
    field!(
        Advisories,
        "maximum-db-staleness",
        advisories_maximum_db_staleness
    );
    field!(
        Advisories,
        "git-fetch-with-cli",
        advisories_git_fetch_with_cli
    );
    field!(
        Advisories,
        "unused-ignored-advisory",
        advisories_unused_ignored_advisory
    );
    field!(Licenses, "version", licenses_version);
    field!(Licenses, "include-dev", licenses_include_dev);
    field!(Licenses, "include-build", licenses_include_build);
    field!(Licenses, "allow", licenses_allow);
    field!(Licenses, "exceptions", licenses_exceptions);
    field!(
        Licenses,
        "confidence-threshold",
        licenses_confidence_threshold
    );
    field!(Licenses, "clarify", licenses_clarify);
    field!(LicensesPrivate, "ignore", licenses_private_ignore);
    field!(LicensesPrivate, "registries", licenses_private_registries);
    field!(
        LicensesPrivate,
        "ignore-sources",
        licenses_private_ignore_sources
    );
    field!(
        Licenses,
        "unused-allowed-license",
        licenses_unused_allowed_license
    );
    field!(
        Licenses,
        "unused-license-exception",
        licenses_unused_license_exception
    );
    field!(Bans, "multiple-versions", bans_multiple_versions);
    field!(
        Bans,
        "multiple-versions-include-dev",
        bans_multiple_versions_include_dev
    );
    field!(Bans, "wildcards", bans_wildcards);
    field!(Bans, "allow-wildcard-paths", bans_allow_wildcard_paths);
    field!(Bans, "highlight", bans_highlight);
    field!(
        Bans,
        "workspace-default-features",
        bans_workspace_default_features
    );
    field!(
        Bans,
        "external-default-features",
        bans_external_default_features
    );
    field!(Bans, "allow", bans_allow);
    field!(Bans, "allow-workspace", bans_allow_workspace);
    field!(Bans, "deny", bans_deny);
    field!(Bans, "features", bans_features);
    field!(Bans, "skip", bans_skip);
    field!(Bans, "skip-tree", bans_skip_tree);
    field!(
        BansWorkspaceDependencies,
        "duplicates",
        bans_workspace_dependencies_duplicates
    );
    field!(
        BansWorkspaceDependencies,
        "include-path-dependencies",
        bans_workspace_dependencies_include_path_dependencies
    );
    field!(
        BansWorkspaceDependencies,
        "unused",
        bans_workspace_dependencies_unused
    );
    field!(BansBuild, "executables", bans_build_executables);
    field!(BansBuild, "interpreted", bans_build_interpreted);
    field!(BansBuild, "script-extensions", bans_build_script_extensions);
    field!(
        BansBuild,
        "enable-builtin-globs",
        bans_build_enable_builtin_globs
    );
    field!(BansBuild, "globs", bans_build_globs);
    field!(
        BansBuild,
        "include-dependencies",
        bans_build_include_dependencies
    );
    field!(BansBuild, "include-workspace", bans_build_include_workspace);
    field!(BansBuild, "include-archives", bans_build_include_archives);
    field!(Sources, "unknown-registry", sources_unknown_registry);
    field!(Sources, "unknown-git", sources_unknown_git);
    field!(Sources, "required-git-spec", sources_required_git_spec);
    field!(Sources, "allow-git", sources_allow_git);
    field!(Sources, "private", sources_private);
    field!(Sources, "allow-registry", sources_allow_registry);
    field!(SourcesAllowOrg, "github", sources_allow_org_github);
    field!(SourcesAllowOrg, "gitlab", sources_allow_org_gitlab);
    field!(SourcesAllowOrg, "bitbucket", sources_allow_org_bitbucket);
    field!(
        Sources,
        "unused-allowed-source",
        sources_unused_allowed_source
    );
    constrain_table_parents(&source.table_keys, &mut requirement);
    requirement
}

fn constrain_table_field(
    requirement: &mut DenyTomlRequirements,
    table: DenyTable,
    file_key: &str,
    value: &impl FileKeyRequirement,
) {
    let keys = requirement.table_keys.entry(table).or_default();
    value.constrain_file_key(file_key, keys);
}

fn constrain_table_parents(
    explicit: &std::collections::BTreeMap<DenyTable, ItemRequirements<KeyedItem<()>>>,
    requirement: &mut DenyTomlRequirements,
) {
    let child_requirements = requirement
        .table_keys
        .iter()
        .chain(explicit)
        .map(|(table, keys)| (*table, keys.clone()))
        .collect::<Vec<_>>();
    for (table, keys) in child_requirements {
        let mut table_presence = ItemRequirements::default();
        keys.constrain_file_key("table", &mut table_presence);
        let messages = table_presence
            .required
            .into_iter()
            .map(|(_, message)| message)
            .collect::<BTreeSet<_>>();
        for message in messages {
            require_table(requirement, table, &message);
        }
    }
}

fn require_table(requirement: &mut DenyTomlRequirements, table: DenyTable, message: &str) {
    let Some((parent, file_key)) = table.parent() else {
        return;
    };
    requirement
        .table_keys
        .entry(parent)
        .or_default()
        .required
        .push((
            KeyedItem {
                file_key: file_key.to_owned(),
                value: (),
            },
            message.to_owned(),
        ));
    require_table(requirement, parent, message);
}

impl DenyTable {
    const fn parent(self) -> Option<(Self, &'static str)> {
        match self {
            Self::Root => None,
            Self::Graph => Some((Self::Root, "graph")),
            Self::Output => Some((Self::Root, "output")),
            Self::Advisories => Some((Self::Root, "advisories")),
            Self::Licenses => Some((Self::Root, "licenses")),
            Self::LicensesPrivate => Some((Self::Licenses, "private")),
            Self::Bans => Some((Self::Root, "bans")),
            Self::BansWorkspaceDependencies => Some((Self::Bans, "workspace-dependencies")),
            Self::BansBuild => Some((Self::Bans, "build")),
            Self::Sources => Some((Self::Root, "sources")),
            Self::SourcesAllowOrg => Some((Self::Sources, "allow-org")),
        }
    }

    const fn finding_key(self) -> &'static str {
        match self {
            Self::Root => "deny.toml",
            Self::Graph => "graph",
            Self::Output => "output",
            Self::Advisories => "advisories",
            Self::Licenses => "licenses",
            Self::LicensesPrivate => "licenses.private",
            Self::Bans => "bans",
            Self::BansWorkspaceDependencies => "bans.workspace-dependencies",
            Self::BansBuild => "bans.build",
            Self::Sources => "sources",
            Self::SourcesAllowOrg => "sources.allow-org",
        }
    }
}
