use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use cargo_metadata::MetadataCommand;

use crate::fs::{canonicalize, cargo_manifests, read_source};
use crate::model::ArchitectureError;

pub(crate) struct ParsedCrate {
    pub crate_name: String,
    pub dependencies: BTreeMap<String, ParsedDependency>,
    pub manifest: PathBuf,
    pub repository_root: PathBuf,
    pub sources: Vec<ParsedSource>,
}

pub(crate) struct ParsedDependency {
    pub default_registry: bool,
    pub package: String,
    pub manifest: Option<PathBuf>,
}

pub(crate) struct ParsedSource {
    pub module_path: Vec<String>,
    pub path: PathBuf,
    pub syntax: syn::File,
}

pub(crate) fn discover(roots: &[PathBuf]) -> Result<Vec<ParsedCrate>, ArchitectureError> {
    let mut crates = Vec::new();
    for root in roots {
        let canonical = canonicalize(root)?;
        for manifest in cargo_manifests(&canonical)? {
            if let Some(parsed) = parse_manifest(&canonical, &manifest)? {
                crates.push(parsed);
            }
        }
    }
    crates.sort_by(|left, right| left.manifest.cmp(&right.manifest));
    Ok(crates)
}

fn parse_manifest(
    repository_root: &Path,
    manifest: &Path,
) -> Result<Option<ParsedCrate>, ArchitectureError> {
    let metadata = MetadataCommand::new()
        .manifest_path(manifest)
        .no_deps()
        .exec()
        .map_err(|error| ArchitectureError::Metadata {
            manifest: manifest.to_path_buf(),
            message: error.to_string(),
        })?;
    let Some(package) = metadata
        .packages
        .iter()
        .find(|package| package.manifest_path.as_std_path() == manifest)
    else {
        return Ok(None);
    };
    let paths = production_sources(package, manifest)?;
    let mut sources = Vec::new();
    for (path, module_path) in paths {
        let text = read_source(&path)?;
        let syntax = syn::parse_file(&text).map_err(|source| ArchitectureError::Parse {
            path: path.clone(),
            source,
        })?;
        sources.push(ParsedSource {
            module_path,
            path,
            syntax,
        });
    }
    Ok(Some(ParsedCrate {
        crate_name: package.name.to_string(),
        dependencies: package
            .dependencies
            .iter()
            .map(|dependency| {
                let local_name = dependency
                    .rename
                    .as_deref()
                    .unwrap_or(&dependency.name)
                    .replace('-', "_");
                (
                    local_name,
                    ParsedDependency {
                        default_registry: dependency.path.is_none()
                            && dependency.registry.is_none()
                            && dependency.source.as_ref().is_some_and(|source| {
                                source.to_string().starts_with(
                                    "registry+https://github.com/rust-lang/crates.io-index",
                                )
                            }),
                        package: dependency.name.clone(),
                        manifest: dependency
                            .path
                            .as_ref()
                            .map(|path| path.as_std_path().join("Cargo.toml")),
                    },
                )
            })
            .collect(),
        manifest: manifest.to_path_buf(),
        repository_root: repository_root.to_path_buf(),
        sources,
    }))
}

fn production_sources(
    package: &cargo_metadata::Package,
    manifest: &Path,
) -> Result<Vec<(PathBuf, Vec<String>)>, ArchitectureError> {
    let package_root = manifest.parent().ok_or_else(|| ArchitectureError::Io {
        path: manifest.to_path_buf(),
        source: std::io::Error::new(std::io::ErrorKind::InvalidInput, "manifest has no parent"),
    })?;
    let mut pending = package
        .targets
        .iter()
        .filter(|target| {
            target.kind.iter().any(|kind| {
                matches!(
                    kind,
                    cargo_metadata::TargetKind::Lib
                        | cargo_metadata::TargetKind::Bin
                        | cargo_metadata::TargetKind::ProcMacro
                        | cargo_metadata::TargetKind::CDyLib
                        | cargo_metadata::TargetKind::DyLib
                        | cargo_metadata::TargetKind::RLib
                        | cargo_metadata::TargetKind::StaticLib
                )
            })
        })
        .map(|target| {
            let path = target.src_path.as_std_path().to_path_buf();
            let module_dir = root_module_directory(package_root, &path);
            (path, module_dir, Vec::new())
        })
        .collect::<Vec<_>>();
    let mut sources = BTreeSet::new();
    while let Some((path, module_dir, module_path)) = pending.pop() {
        let path = canonicalize(&path)?;
        if !sources.insert((path.clone(), module_path.clone())) {
            continue;
        }
        let syntax =
            syn::parse_file(&read_source(&path)?).map_err(|source| ArchitectureError::Parse {
                path: path.clone(),
                source,
            })?;
        collect_external_modules(&syntax.items, &module_dir, &module_path, &mut pending);
    }
    Ok(sources.into_iter().collect())
}

fn root_module_directory(package_root: &Path, source: &Path) -> PathBuf {
    let parent = source.parent().unwrap_or(package_root);
    match source.file_stem().and_then(|name| name.to_str()) {
        Some("lib" | "main" | "mod") | None => parent.to_path_buf(),
        Some(stem) => parent.join(stem),
    }
}

fn collect_external_modules(
    items: &[syn::Item],
    module_dir: &Path,
    module_path: &[String],
    pending: &mut Vec<(PathBuf, PathBuf, Vec<String>)>,
) {
    for item in items {
        let syn::Item::Mod(module) = item else {
            continue;
        };
        if let Some((_, nested)) = &module.content {
            let mut nested_path = module_path.to_vec();
            nested_path.push(module.ident.to_string());
            collect_external_modules(
                nested,
                &module_dir.join(module.ident.to_string()),
                &nested_path,
                pending,
            );
            continue;
        }
        let explicit = module.attrs.iter().find_map(|attribute| {
            let syn::Meta::NameValue(meta) = &attribute.meta else {
                return None;
            };
            if !meta.path.is_ident("path") {
                return None;
            }
            let syn::Expr::Lit(syn::ExprLit {
                lit: syn::Lit::Str(path),
                ..
            }) = &meta.value
            else {
                return None;
            };
            Some(path.clone())
        });
        let path = explicit.map_or_else(
            || {
                let flat = module_dir.join(format!("{}.rs", module.ident));
                if flat.is_file() {
                    flat
                } else {
                    module_dir.join(module.ident.to_string()).join("mod.rs")
                }
            },
            |relative| module_dir.join(relative.value()),
        );
        if path.is_file() {
            let mut external_path = module_path.to_vec();
            external_path.push(module.ident.to_string());
            pending.push((
                path.clone(),
                root_module_directory(module_dir, &path),
                external_path,
            ));
        }
    }
}
