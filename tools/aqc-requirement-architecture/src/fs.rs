#![expect(
    clippy::disallowed_methods,
    reason = "This module is the tool's centralized filesystem boundary."
)]

use std::path::{Path, PathBuf};

use crate::model::ArchitectureError;

const EXCLUDED_DIRECTORIES: &[&str] = &[
    ".fixture3",
    ".cargo",
    ".cargo-target",
    ".git",
    ".plans",
    ".worklogs",
    "fixtures",
    "node_modules",
    "registry",
    "specs",
    "target",
    "tests",
    "vendor",
];

pub(crate) fn canonicalize(path: &Path) -> Result<PathBuf, ArchitectureError> {
    std::fs::canonicalize(path).map_err(|source| ArchitectureError::Io {
        path: path.to_path_buf(),
        source,
    })
}

pub(crate) fn cargo_manifests(root: &Path) -> Result<Vec<PathBuf>, ArchitectureError> {
    let mut manifests = Vec::new();
    walk_manifests(root, root, &mut manifests)?;
    manifests.sort();
    manifests.dedup();
    Ok(manifests)
}

fn walk_manifests(
    root: &Path,
    directory: &Path,
    manifests: &mut Vec<PathBuf>,
) -> Result<(), ArchitectureError> {
    let entries = std::fs::read_dir(directory).map_err(|source| ArchitectureError::Io {
        path: directory.to_path_buf(),
        source,
    })?;
    for entry in entries {
        let entry = entry.map_err(|source| ArchitectureError::Io {
            path: directory.to_path_buf(),
            source,
        })?;
        let path = entry.path();
        if path.is_dir() {
            if !is_excluded(root, &path) {
                walk_manifests(root, &path, manifests)?;
            }
        } else if path.file_name().is_some_and(|name| name == "Cargo.toml") {
            manifests.push(path);
        }
    }
    Ok(())
}

fn is_excluded(root: &Path, path: &Path) -> bool {
    path.strip_prefix(root).is_ok_and(|relative| {
        relative.components().any(|component| {
            let name = component.as_os_str().to_string_lossy();
            EXCLUDED_DIRECTORIES.contains(&name.as_ref())
        })
    })
}

pub(crate) fn read_source(path: &Path) -> Result<String, ArchitectureError> {
    std::fs::read_to_string(path).map_err(|source| ArchitectureError::Io {
        path: path.to_path_buf(),
        source,
    })
}
