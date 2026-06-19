//! The two-phase walk over the `ignore` crate's walker.

use std::collections::BTreeMap;
use std::path::Path;

use ignore::WalkBuilder;

use crate::fs;
use crate::options::{RecoveryRules, SymlinkPolicy, WalkError, WalkOptions};
use crate::tree::{EntryOrigin, FileEntry, FileKind, FileTree};

/// Walk `root` per `options` and return the sorted [`FileTree`].
///
/// # Errors
///
/// [`WalkError::RootNotFound`] / [`WalkError::RootNotADirectory`] /
/// [`WalkError::Io`].
pub fn build_file_tree(
    root: impl AsRef<Path>,
    options: &WalkOptions,
) -> Result<FileTree, WalkError> {
    let root = fs::checked_root(root.as_ref())?;

    let mut entries: BTreeMap<String, FileEntry> = BTreeMap::new();
    walk_phase(
        &root,
        options,
        options.respect_gitignore,
        &mut entries,
        None,
    )?;
    if let Some(rules) = &options.recovery {
        walk_phase(&root, options, false, &mut entries, Some(rules))?;
    }
    Ok(FileTree::new(root, entries.into_values().collect()))
}

/// One walk pass. With `rules = None` this is phase 1 (`origin = Primary`);
/// with rules it is phase 2 (no gitignore, `origin = Recovered`, only
/// rule-matching paths are added, already-present paths are skipped).
fn walk_phase(
    root: &Path,
    options: &WalkOptions,
    respect_gitignore: bool,
    entries: &mut BTreeMap<String, FileEntry>,
    rules: Option<&RecoveryRules>,
) -> Result<(), WalkError> {
    let mut builder = WalkBuilder::new(root);
    let _ = builder
        .hidden(!options.include_hidden)
        .git_ignore(respect_gitignore)
        .ignore(respect_gitignore)
        .git_global(false)
        .git_exclude(respect_gitignore)
        .require_git(false)
        .follow_links(options.symlink_policy == SymlinkPolicy::Follow)
        .sort_by_file_name(std::cmp::Ord::cmp);
    if let Some(depth) = options.max_depth {
        let _ = builder.max_depth(Some(usize::try_from(depth).unwrap_or(usize::MAX)));
    }
    let skip_names = options.skip_dir_names.clone();
    let root_for_filter = root.to_path_buf();
    let skip_prefixes = options.skip_path_prefixes.clone();
    let _ = builder.filter_entry(move |entry| {
        let is_dir = entry.file_type().is_some_and(|t| t.is_dir());
        let name = entry.file_name().to_string_lossy();
        if is_dir && skip_names.iter().any(|s| s.as_str() == name) {
            return false;
        }
        let under_skipped_prefix = entry
            .path()
            .strip_prefix(&root_for_filter)
            .is_ok_and(|rel| {
                let rel = rel.to_string_lossy().replace('\\', "/");
                skip_prefixes
                    .iter()
                    .any(|p| rel == *p || rel.starts_with(&format!("{p}/")))
            });
        if !skip_prefixes.is_empty() && under_skipped_prefix {
            return false;
        }
        true
    });

    for result in builder.build() {
        let entry = match result {
            Ok(entry) => entry,
            // Declared policy: a per-entry error aborts the whole walk.
            Err(source) => {
                return Err(WalkError::Io {
                    path: root.to_path_buf(),
                    source: std::io::Error::other(source),
                });
            }
        };
        record_entry(root, options, entries, rules, &entry);
    }
    Ok(())
}

/// Classify one walked entry and insert it unless skipped (root, symlink
/// policy, already present, or phase-2 rules say no).
fn record_entry(
    root: &Path,
    options: &WalkOptions,
    entries: &mut BTreeMap<String, FileEntry>,
    rules: Option<&RecoveryRules>,
    entry: &ignore::DirEntry,
) {
    if entry.depth() == 0 {
        return; // the root itself
    }
    let Ok(rel) = entry.path().strip_prefix(root) else {
        return;
    };
    let rel_path = rel.to_string_lossy().replace('\\', "/");
    let kind = if entry.path_is_symlink() {
        match options.symlink_policy {
            SymlinkPolicy::Skip => return,
            SymlinkPolicy::Record => FileKind::Symlink,
            SymlinkPolicy::Follow => {
                if entry
                    .file_type()
                    .is_some_and(|file_type| file_type.is_dir())
                {
                    FileKind::Directory
                } else {
                    FileKind::File
                }
            }
        }
    } else if entry.file_type().is_some_and(|t| t.is_dir()) {
        FileKind::Directory
    } else {
        FileKind::File
    };
    if entries.contains_key(&rel_path) {
        return;
    }
    let name = entry.file_name().to_string_lossy();
    let (origin, keep) = rules.map_or((EntryOrigin::Primary, true), |rules| {
        (
            EntryOrigin::Recovered,
            recovery_rules_match(rules, &rel_path, &name, kind),
        )
    });
    if !keep {
        return;
    }
    let _ = entries.insert(
        rel_path.clone(),
        FileEntry {
            rel_path,
            abs_path: entry.path().to_path_buf(),
            kind,
            origin,
        },
    );
}

/// Return whether a phase-2 walk entry should be recovered.
fn recovery_rules_match(rules: &RecoveryRules, rel_path: &str, name: &str, kind: FileKind) -> bool {
    match kind {
        FileKind::Directory => rules.directory_names.iter().any(|dir| dir == name),
        FileKind::File | FileKind::Symlink => {
            rules.exact_file_names.iter().any(|file| file == name)
                || rules
                    .file_name_prefixes
                    .iter()
                    .any(|prefix| name.starts_with(prefix))
                || rules
                    .rel_path_suffixes
                    .iter()
                    .any(|suffix| rel_path.ends_with(suffix))
        }
    }
}
