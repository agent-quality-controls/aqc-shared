//! Finding and attribution support for pnpm YAML reconciliation.

use std::collections::BTreeSet;

use aqc_file_engine_core::{
    Finding, KeyedItem, Provenance, ResolvedForbiddenGlobRequirements, ResolvedItemRequirements,
    ResolvedListRequirements, ResolvedRequirement, Severity,
};
use globset::GlobMatcher;

use crate::runtime::compile_selector_glob;
use crate::types::PnpmPackageSelectorGlob;

pub(super) struct CompiledForbiddenGlob {
    pub(super) matcher: GlobMatcher,
    message: String,
    attribution: Vec<Provenance>,
}

pub(super) fn push_forbidden_selector_findings(
    key: &str,
    selectors: &[String],
    globs: &[CompiledForbiddenGlob],
    findings: &mut Vec<Finding>,
) {
    let mut seen = BTreeSet::new();
    for selector in selectors {
        if seen.insert(selector) {
            push_glob_item_finding(key, selector, globs, findings);
        }
    }
}

pub(super) fn push_glob_item_finding(
    key: &str,
    selector: &str,
    globs: &[CompiledForbiddenGlob],
    findings: &mut Vec<Finding>,
) {
    let mut matches = globs.iter().filter(|glob| glob.matcher.is_match(selector));
    let Some(first) = matches.next() else {
        return;
    };
    let mut attribution = first.attribution.clone();
    for matched in matches {
        attribution.extend(matched.attribution.iter().cloned());
    }
    attribution.sort();
    attribution.dedup();
    push_collection_mismatch(
        key,
        first.message.clone(),
        attribution,
        Some(selector.to_owned()),
        findings,
    );
}

pub(super) fn compile_globs(
    key: &str,
    globs: &ResolvedForbiddenGlobRequirements<PnpmPackageSelectorGlob>,
    findings: &mut Vec<Finding>,
) -> Vec<CompiledForbiddenGlob> {
    let mut compiled = Vec::new();
    for glob in globs.globs.values() {
        match compile_selector_glob(&glob.merged.glob) {
            Ok(matcher) => compiled.push(CompiledForbiddenGlob {
                matcher,
                message: glob
                    .collected
                    .first()
                    .map_or_else(String::new, |(_, message)| message.clone()),
                attribution: glob.attribution(),
            }),
            Err(error) => findings.push(Finding::InvalidRequirements {
                key: key.to_owned(),
                message: format!(
                    "invalid pnpm package-selector glob '{}': {error}",
                    glob.merged.glob
                ),
                contributors: glob
                    .collected
                    .iter()
                    .map(|(provenance, message)| (provenance.policy.clone(), message.clone()))
                    .collect(),
            }),
        }
    }
    compiled
}

pub(super) fn push_shape_finding(
    key: &str,
    attribution: Vec<Provenance>,
    findings: &mut Vec<Finding>,
) {
    push_collection_mismatch(
        key,
        "the YAML field has the wrong shape".to_owned(),
        attribution,
        None,
        findings,
    );
}

pub(super) fn push_collection_mismatch(
    key: &str,
    message: String,
    attribution: Vec<Provenance>,
    selector: Option<String>,
    findings: &mut Vec<Finding>,
) {
    findings.push(Finding::Mismatch {
        key: key.to_owned(),
        selector,
        current: Some("configured value".to_owned()),
        expected: "resolved requirement".to_owned(),
        message,
        severity: Severity::Error,
        attribution,
    });
}

pub(super) fn list_attribution(
    requirement: &ResolvedListRequirements,
    globs: &ResolvedForbiddenGlobRequirements<PnpmPackageSelectorGlob>,
) -> Vec<Provenance> {
    let mut out = requirement
        .contains
        .values()
        .flat_map(ResolvedRequirement::attribution)
        .collect::<Vec<_>>();
    out.extend(
        requirement
            .excludes
            .values()
            .flat_map(ResolvedRequirement::attribution),
    );
    out.extend(
        requirement
            .exact
            .iter()
            .flat_map(|item| item.collected.iter().map(|(p, _)| p.clone())),
    );
    out.extend(
        globs
            .globs
            .values()
            .flat_map(ResolvedRequirement::attribution),
    );
    out.sort();
    out.dedup();
    out
}

pub(super) fn item_attribution(
    requirement: &ResolvedItemRequirements<KeyedItem<bool>>,
    globs: &ResolvedForbiddenGlobRequirements<PnpmPackageSelectorGlob>,
) -> Vec<Provenance> {
    let mut out = requirement
        .required
        .values()
        .flat_map(ResolvedRequirement::attribution)
        .collect::<Vec<_>>();
    out.extend(
        requirement
            .forbidden
            .values()
            .flat_map(ResolvedRequirement::attribution),
    );
    out.extend(
        globs
            .globs
            .values()
            .flat_map(ResolvedRequirement::attribution),
    );
    out.extend(
        requirement
            .exact
            .iter()
            .flat_map(|item| item.collected.iter().map(|(p, _)| p.clone())),
    );
    out.sort();
    out.dedup();
    out
}

pub(super) fn list_message(requirement: &ResolvedListRequirements) -> String {
    requirement
        .contains
        .values()
        .flat_map(|item| item.collected.iter().map(|(_, message)| message.clone()))
        .chain(
            requirement
                .excludes
                .values()
                .flat_map(|item| item.collected.iter().map(|(_, message)| message.clone())),
        )
        .chain(requirement.exact.iter().flat_map(|item| {
            item.collected
                .iter()
                .map(|(_, (_, message))| message.clone())
        }))
        .next()
        .unwrap_or_else(|| "list does not satisfy the resolved requirement".to_owned())
}
