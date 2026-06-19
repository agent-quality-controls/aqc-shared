//! Behavior probes for the two-phase walk contract in `plan.md`.

#![expect(
    clippy::expect_used,
    reason = "Fixture helpers outside #[test] fns assert setup success; a failed expect is the test failing."
)]
#![expect(
    clippy::disallowed_methods,
    reason = "Test fixtures write real files/dirs on purpose; the crate under test is the sanctioned fs layer."
)]

// Lib dependencies linked into the test build but exercised only through the
// crate under test.
use globset as _;
use ignore as _;

mod walk {
    use std::fs;
    use std::path::Path;

    use aqc_filetree::{
        EntryOrigin, FileKind, RecoveryRules, SkipDirPreset, SymlinkPolicy, WalkError, WalkOptions,
        build_file_tree,
    };

    /// A repo-shaped fixture tree: `src/lib.rs`, `.gitignore` (ignoring
    /// `ignored-dir/`), `ignored-dir/deep/Cargo.toml`,
    /// `target/debug/Cargo.toml`, `.hidden/h.txt`.
    fn fixture() -> tempfile::TempDir {
        let dir = tempfile::tempdir().expect("fixture must create a temp dir");
        let root = dir.path();
        fs::create_dir_all(root.join("src")).expect("fixture must create src/");
        fs::write(root.join("src/lib.rs"), "x").expect("fixture must write src/lib.rs");
        fs::write(root.join(".gitignore"), "ignored-dir/\n")
            .expect("fixture must write the .gitignore");
        fs::create_dir_all(root.join("ignored-dir/deep"))
            .expect("fixture must create the gitignored tree");
        fs::write(root.join("ignored-dir/deep/Cargo.toml"), "x")
            .expect("fixture must write the gitignored manifest");
        fs::create_dir_all(root.join("target/debug")).expect("fixture must create target/debug");
        fs::write(root.join("target/debug/Cargo.toml"), "x")
            .expect("fixture must write the target-tree manifest");
        fs::create_dir_all(root.join(".hidden")).expect("fixture must create the hidden dir");
        fs::write(root.join(".hidden/h.txt"), "x").expect("fixture must write the hidden file");
        dir
    }

    fn rel_paths(tree: &aqc_filetree::FileTree) -> Vec<&str> {
        tree.entries().iter().map(|e| e.rel_path.as_str()).collect()
    }

    #[test]
    fn phase1_respects_gitignore_and_skip_dirs_and_lists_hidden() {
        let dir = fixture();
        let tree = build_file_tree(dir.path(), &WalkOptions::default())
            .expect("the walk must succeed on the fixture tree");
        let paths = rel_paths(&tree);
        assert!(paths.contains(&"src/lib.rs"), "{paths:?}");
        assert!(
            paths.contains(&".hidden/h.txt"),
            "hidden included: {paths:?}"
        );
        assert!(
            !paths.iter().any(|p| p.starts_with("ignored-dir")),
            "gitignored tree excluded: {paths:?}"
        );
        assert!(
            !paths.iter().any(|p| p.starts_with("target")),
            "skip_dir_names prunes target/: {paths:?}"
        );
        assert!(
            tree.entries().windows(2).all(|w| match w {
                [a, b] => a.rel_path < b.rel_path,
                _ => true,
            }),
            "entries sorted by rel_path"
        );
        assert!(
            tree.entries()
                .iter()
                .all(|e| e.origin == EntryOrigin::Primary),
            "phase 1 only"
        );
    }

    #[test]
    fn phase2_recovers_named_files_from_ignored_trees_without_double_listing() {
        let dir = fixture();
        let options = WalkOptions {
            recovery: Some(RecoveryRules {
                exact_file_names: vec!["Cargo.toml".to_owned()],
                ..RecoveryRules::default()
            }),
            ..WalkOptions::default()
        };
        let tree = build_file_tree(dir.path(), &options)
            .expect("the walk must succeed on the fixture tree");
        let recovered = tree.entries_with_origin(EntryOrigin::Recovered);
        let recovered_paths: Vec<&str> = recovered.iter().map(|e| e.rel_path.as_str()).collect();
        assert_eq!(
            recovered_paths,
            vec!["ignored-dir/deep/Cargo.toml"],
            "recovers from the gitignored tree but NOT from skip_dir_names (target/ stays pruned in both phases)"
        );
        assert!(
            !recovered_paths.contains(&"src/lib.rs"),
            "phase 1 entries are not re-listed"
        );
        let entry = tree
            .entry("ignored-dir/deep/Cargo.toml")
            .expect("the entry query must find the recovered path");
        assert_eq!(entry.kind, FileKind::File);
    }

    #[test]
    fn glob_query() {
        let dir = fixture();
        let tree = build_file_tree(dir.path(), &WalkOptions::default())
            .expect("the walk must succeed on the fixture tree");
        let hits = tree
            .glob("**/*.rs", true)
            .expect("the glob pattern must be valid");
        assert_eq!(hits.len(), 1, "one .rs file");
        assert_eq!(
            hits.first().map(|e| e.rel_path.as_str()),
            Some("src/lib.rs")
        );
    }

    #[test]
    fn max_depth_limits_descent() {
        let dir = fixture();
        let options = WalkOptions {
            max_depth: Some(1),
            ..WalkOptions::default()
        };
        let tree = build_file_tree(dir.path(), &options)
            .expect("the walk must succeed on the fixture tree");
        assert!(
            tree.entries().iter().all(|e| !e.rel_path.contains('/')),
            "depth 1 lists only top-level entries: {:?}",
            rel_paths(&tree)
        );
    }

    #[test]
    fn skip_path_prefixes_prunes_subtrees() {
        let dir = fixture();
        let options = WalkOptions {
            skip_path_prefixes: vec!["src".to_owned()],
            ..WalkOptions::default()
        };
        let tree = build_file_tree(dir.path(), &options)
            .expect("the walk must succeed on the fixture tree");
        assert!(
            !rel_paths(&tree).iter().any(|p| p.starts_with("src")),
            "{:?}",
            rel_paths(&tree)
        );
    }

    #[test]
    fn root_errors() {
        let dir = fixture();
        let missing = build_file_tree(dir.path().join("absent"), &WalkOptions::default());
        assert!(
            matches!(missing, Err(WalkError::RootNotFound)),
            "{missing:?}"
        );
        let file_root = build_file_tree(dir.path().join(".gitignore"), &WalkOptions::default());
        assert!(
            matches!(file_root, Err(WalkError::RootNotADirectory)),
            "{file_root:?}"
        );
    }

    #[cfg(unix)]
    #[test]
    fn symlink_policies() {
        let dir = fixture();
        let root = dir.path();
        std::os::unix::fs::symlink(Path::new("src/lib.rs"), root.join("link.rs"))
            .expect("fixture must create the symlink");

        let skipped = build_file_tree(root, &WalkOptions::default())
            .expect("the walk must succeed on the fixture tree");
        assert!(skipped.entry("link.rs").is_none(), "Skip: not listed");

        let recorded = build_file_tree(
            root,
            &WalkOptions {
                symlink_policy: SymlinkPolicy::Record,
                ..WalkOptions::default()
            },
        )
        .expect("the walk must succeed on the fixture tree");
        assert_eq!(
            recorded.entry("link.rs").map(|e| e.kind),
            Some(FileKind::Symlink),
            "Record: listed as Symlink"
        );
    }

    #[test]
    fn preset_merge_dedupes() {
        let merged = SkipDirPreset::merge(&[
            SkipDirPreset::Common,
            SkipDirPreset::Rust,
            SkipDirPreset::Rust,
        ]);
        assert_eq!(merged, vec![".git".to_owned(), "target".to_owned()]);
    }
}
