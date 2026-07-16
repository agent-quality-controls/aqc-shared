use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use cargo_metadata::MetadataCommand;

use crate::fs::{canonicalize, cargo_manifests, read_source, rust_sources};
use crate::model::ArchitectureError;

pub(crate) struct ParsedCrate {
    pub crate_name: String,
    pub manifest: PathBuf,
    pub repository_root: PathBuf,
    pub sources: Vec<ParsedSource>,
}

pub(crate) struct ParsedSource {
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
    let mut source_directories = BTreeSet::new();
    for target in &package.targets {
        if let Some(directory) =
            production_source_directory(manifest, target.src_path.as_std_path())
        {
            source_directories.insert(directory);
        }
    }
    let mut paths = BTreeSet::new();
    for directory in source_directories {
        paths.extend(rust_sources(&directory)?);
    }
    let mut sources = Vec::new();
    for path in paths {
        let text = read_source(&path)?;
        let syntax = syn::parse_file(&text).map_err(|source| ArchitectureError::Parse {
            path: path.clone(),
            source,
        })?;
        sources.push(ParsedSource { path, syntax });
    }
    Ok(Some(ParsedCrate {
        crate_name: package.name.to_string(),
        manifest: manifest.to_path_buf(),
        repository_root: repository_root.to_path_buf(),
        sources,
    }))
}

fn production_source_directory(manifest: &Path, source: &Path) -> Option<PathBuf> {
    let package_root = manifest.parent()?;
    let relative = source.strip_prefix(package_root).ok()?;
    let first = relative.components().next()?.as_os_str();
    (first == "src").then(|| package_root.join("src"))
}
