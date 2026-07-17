#![allow(
    clippy::as_conversions,
    clippy::missing_const_for_fn,
    clippy::type_complexity,
    reason = "Shared test helpers keep compact Cargo requirement fixture shapes."
)]

use aqc_toml_engine_core as _;
pub(crate) use std::collections::{BTreeMap, BTreeSet};

pub(crate) use aqc_cargo_toml_engine as cargo;
pub(crate) use aqc_file_engine_core as engine_core;
use engine_core::Engine;
use globset as _;
use toml_edit as _;

#[derive(Debug, Clone)]
pub(crate) struct KeyedFixture<Entry> {
    pub(crate) required: BTreeMap<String, (Entry, String)>,
    pub(crate) forbidden: BTreeMap<String, String>,
    pub(crate) exact: Option<String>,
}

pub(crate) fn prov(policy: &str) -> engine_core::Provenance {
    engine_core::Provenance {
        policy: policy.to_owned(),
    }
}

pub(crate) fn normal_scope() -> cargo::DependencyScope {
    cargo::DependencyScope {
        kind: cargo::DependencyKind::Normal,
        target: None,
    }
}

pub(crate) fn unix_scope() -> cargo::DependencyScope {
    cargo::DependencyScope {
        kind: cargo::DependencyKind::Normal,
        target: Some("cfg(unix)".to_owned()),
    }
}

pub(crate) fn dep_spec(version: Option<&str>) -> cargo::DependencySpec {
    cargo::DependencySpec {
        version: version.map(str::to_owned),
        ..cargo::DependencySpec::default()
    }
}

pub(crate) fn dep_req(table: KeyedFixture<cargo::DependencySpec>) -> cargo::CargoTomlRequirements {
    let mut req = cargo::CargoTomlRequirements::default();
    let _ = req.dependencies.insert(normal_scope(), dep_items(table));
    req
}

pub(crate) fn dep_item_req(
    items: engine_core::ItemRequirements<cargo::DependencyRequirement>,
) -> cargo::CargoTomlRequirements {
    let mut req = cargo::CargoTomlRequirements::default();
    let _ = req.dependencies.insert(normal_scope(), items);
    req
}

pub(crate) fn dependency_package_glob(glob: &str) -> cargo::DependencyPackageGlob {
    cargo::DependencyPackageGlob {
        glob: glob.to_owned(),
    }
}

pub(crate) fn dependency_package_globs(
    globs: Vec<(&str, &str)>,
) -> engine_core::ForbiddenGlobRequirements<cargo::DependencyPackageGlob> {
    engine_core::ForbiddenGlobRequirements {
        globs: globs
            .into_iter()
            .map(|(glob, msg)| (dependency_package_glob(glob), msg.to_owned()))
            .collect(),
    }
}

pub(crate) fn dep_glob_req(globs: Vec<(&str, &str)>) -> cargo::CargoTomlRequirements {
    let mut req = cargo::CargoTomlRequirements::default();
    let _ = req
        .forbidden_dependency_package_globs
        .insert(normal_scope(), dependency_package_globs(globs));
    req
}

pub(crate) fn package_requirement(
    package: &str,
    version: Option<&str>,
) -> cargo::DependencyRequirement {
    cargo::DependencyRequirement {
        file_key: None,
        value: cargo::DependencySpec {
            package: Some(package.to_owned()),
            version: version.map(str::to_owned),
            ..cargo::DependencySpec::default()
        },
    }
}

pub(crate) fn local_dependency_requirement(
    file_key: &str,
    package: Option<&str>,
    version: Option<&str>,
) -> cargo::DependencyRequirement {
    cargo::DependencyRequirement {
        file_key: Some(file_key.to_owned()),
        value: cargo::DependencySpec {
            package: package.map(str::to_owned),
            version: version.map(str::to_owned),
            ..cargo::DependencySpec::default()
        },
    }
}

pub(crate) fn dep_items(
    table: KeyedFixture<cargo::DependencySpec>,
) -> engine_core::ItemRequirements<cargo::DependencyRequirement> {
    let exact = table.exact.map(|message| {
        (
            table
                .required
                .iter()
                .map(|(file_key, (value, _))| cargo::DependencyRequirement {
                    file_key: Some(file_key.clone()),
                    value: value.clone(),
                })
                .collect(),
            message,
        )
    });
    engine_core::ItemRequirements {
        required: table
            .required
            .into_iter()
            .map(|(file_key, (value, msg))| {
                (
                    cargo::DependencyRequirement {
                        file_key: Some(file_key),
                        value,
                    },
                    msg,
                )
            })
            .collect(),
        forbidden: table
            .forbidden
            .into_iter()
            .map(|(file_key, msg)| {
                (
                    cargo::DependencyRequirement {
                        file_key: Some(file_key),
                        value: cargo::DependencySpec::default(),
                    },
                    msg,
                )
            })
            .collect(),
        allowed: None,
        exact,
    }
}

pub(crate) fn keyed_items<Entry: Default + Clone>(
    table: KeyedFixture<Entry>,
) -> engine_core::ItemRequirements<engine_core::KeyedItem<Entry>> {
    let exact = table.exact.map(|message| {
        (
            table
                .required
                .iter()
                .map(|(file_key, (value, _))| engine_core::KeyedItem {
                    file_key: file_key.clone(),
                    value: value.clone(),
                })
                .collect(),
            message,
        )
    });
    engine_core::ItemRequirements {
        required: table
            .required
            .into_iter()
            .map(|(file_key, (value, msg))| (engine_core::KeyedItem { file_key, value }, msg))
            .collect(),
        forbidden: table
            .forbidden
            .into_iter()
            .map(|(file_key, msg)| {
                (
                    engine_core::KeyedItem {
                        file_key,
                        value: Entry::default(),
                    },
                    msg,
                )
            })
            .collect(),
        allowed: None,
        exact,
    }
}

pub(crate) fn cargo_findings(
    reqs: Vec<(engine_core::Provenance, cargo::CargoTomlRequirements)>,
) -> Vec<engine_core::Finding> {
    cargo_findings_with(Some(b""), reqs)
}

pub(crate) fn cargo_findings_with(
    bytes: Option<&[u8]>,
    reqs: Vec<(engine_core::Provenance, cargo::CargoTomlRequirements)>,
) -> Vec<engine_core::Finding> {
    let reqs = reqs
        .into_iter()
        .map(|(p, r)| (p, Box::new(r) as Box<dyn engine_core::EngineRequirement>))
        .collect::<Vec<_>>();
    cargo_output_from_erased(bytes, &reqs).findings
}

pub(crate) fn cargo_output(
    bytes: Option<&[u8]>,
    reqs: Vec<(engine_core::Provenance, cargo::CargoTomlRequirements)>,
) -> engine_core::EngineOutput {
    let reqs = reqs
        .into_iter()
        .map(|(p, r)| (p, Box::new(r) as Box<dyn engine_core::EngineRequirement>))
        .collect::<Vec<_>>();
    cargo_output_from_erased(bytes, &reqs)
}

fn cargo_output_from_erased(
    bytes: Option<&[u8]>,
    reqs: &[(
        engine_core::Provenance,
        Box<dyn engine_core::EngineRequirement>,
    )],
) -> engine_core::EngineOutput {
    cargo::CargoTomlEngine.reconcile(bytes, reqs)
}

pub(crate) fn has_conflict(findings: &[engine_core::Finding]) -> bool {
    findings
        .iter()
        .any(|f| matches!(f, engine_core::Finding::ConflictingRequirements { .. }))
}

pub(crate) fn mismatch_count_for_key(findings: &[engine_core::Finding], wanted: &str) -> usize {
    findings
        .iter()
        .filter(|finding| matches!(finding, engine_core::Finding::Mismatch { key, .. } if key == wanted))
        .count()
}

#[test]
fn common_helpers_compile() {
    let _set: BTreeSet<String> = BTreeSet::new();
    let _ = prov("policy");
    let _ = normal_scope();
    let _ = unix_scope();
    let _ = dep_spec(Some("1"));
    let _ = dep_req(KeyedFixture {
        required: BTreeMap::new(),
        forbidden: BTreeMap::new(),
        exact: None,
    });
    let _ = dep_item_req(engine_core::ItemRequirements {
        required: Vec::new(),
        forbidden: Vec::new(),
        allowed: None,
        exact: None,
    });
    let _ = dependency_package_glob("*");
    let _ = dependency_package_globs(Vec::new());
    let _ = dep_glob_req(Vec::new());
    let _ = package_requirement("serde", Some("1"));
    let _ = local_dependency_requirement("serde", None, None);
    let _ = dep_items(KeyedFixture {
        required: BTreeMap::new(),
        forbidden: BTreeMap::new(),
        exact: None,
    });
    let _ = keyed_items::<cargo::DependencySpec>(KeyedFixture {
        required: BTreeMap::new(),
        forbidden: BTreeMap::new(),
        exact: None,
    });
    let _ = cargo_findings(Vec::new());
    let _ = cargo_findings_with(Some(b""), Vec::new());
    let _ = cargo_output(None, Vec::new());
    let _ = has_conflict(&[]);
    let _ = mismatch_count_for_key(&[], "key");
}
