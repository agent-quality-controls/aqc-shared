#![expect(
    clippy::disallowed_methods,
    reason = "This module is the tool's centralized filesystem boundary."
)]

use std::path::{Path, PathBuf};
use std::process::Command;

use crate::model::ArchitectureError;

const EXCLUDED_ANYWHERE: &[&str] = &["target"];

const EXCLUDED_ROOT_DIRECTORIES: &[&str] = &[
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
    add_tracked_target_manifests(root, &mut manifests);
    manifests.sort();
    manifests.dedup();
    Ok(manifests)
}

fn add_tracked_target_manifests(root: &Path, manifests: &mut Vec<PathBuf>) {
    let Ok(output) = Command::new("git")
        .args(["ls-files", "--cached", "--", ":(glob)**/Cargo.toml"])
        .current_dir(root)
        .output()
    else {
        return;
    };
    if !output.status.success() {
        return;
    }
    for relative in String::from_utf8_lossy(&output.stdout).lines() {
        let path = Path::new(relative);
        if path
            .components()
            .any(|component| component.as_os_str() == "target")
            && !is_checker_fixture_path(path)
        {
            manifests.push(root.join(path));
        }
    }
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
    let Ok(relative) = path.strip_prefix(root) else {
        return false;
    };
    let components = relative
        .components()
        .map(|component| component.as_os_str().to_string_lossy())
        .collect::<Vec<_>>();
    let first_is_root_exclusion = components
        .first()
        .is_some_and(|name| EXCLUDED_ROOT_DIRECTORIES.contains(&name.as_ref()));
    let is_checker_fixture = is_checker_fixture_path(relative);
    first_is_root_exclusion
        || is_checker_fixture
        || components
            .iter()
            .any(|name| EXCLUDED_ANYWHERE.contains(&name.as_ref()))
}

fn is_checker_fixture_path(path: &Path) -> bool {
    let components = path
        .components()
        .map(|component| component.as_os_str().to_string_lossy())
        .collect::<Vec<_>>();
    components.len() >= 4
        && components[0] == "tools"
        && components[1] == "aqc-requirement-architecture"
        && components[2] == "tests"
        && components[3] == "fixtures"
}

pub(crate) fn read_source(path: &Path) -> Result<String, ArchitectureError> {
    std::fs::read_to_string(path).map_err(|source| ArchitectureError::Io {
        path: path.to_path_buf(),
        source,
    })
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::is_excluded;

    #[test]
    fn production_crates_cannot_hide_below_generic_support_names() {
        let root = Path::new("/repo");
        assert!(!is_excluded(root, Path::new("/repo/tests/hidden")));
        assert!(!is_excluded(root, Path::new("/repo/packages/specs/hidden")));
        assert!(!is_excluded(
            root,
            Path::new("/repo/packages/fixtures/hidden")
        ));
        assert!(!is_excluded(
            root,
            Path::new("/repo/packages/tools/aqc-requirement-architecture/tests/fixtures/hidden")
        ));
        assert!(!is_excluded(
            root,
            Path::new("/repo/packages/vendor/hidden")
        ));
        assert!(!is_excluded(
            root,
            Path::new("/repo/packages/registry/hidden")
        ));
        assert!(!is_excluded(
            root,
            Path::new("/repo/packages/node_modules/hidden")
        ));
    }

    #[test]
    fn repository_outputs_and_checker_fixtures_are_excluded() {
        let root = Path::new("/repo");
        assert!(is_excluded(root, Path::new("/repo/.cargo-target/build")));
        assert!(is_excluded(root, Path::new("/repo/specs")));
        assert!(is_excluded(
            root,
            Path::new("/repo/tools/aqc-requirement-architecture/tests/fixtures/case/target/hidden")
        ));
    }
}
