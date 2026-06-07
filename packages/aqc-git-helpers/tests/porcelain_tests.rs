//! Parser probes on fixture porcelain byte strings (no repo needed) plus one
//! temp `git init` integration probe.

#![expect(
    clippy::type_complexity,
    reason = "Vec<(&str, ChangeStatus)> assertion shapes; declared openly."
)]
#![expect(
    clippy::disallowed_methods,
    reason = "Test fixtures write real files/dirs on purpose; the crate under test is the sanctioned fs layer."
)]

mod porcelain {
    use aqc_git_helpers::{
        ChangeStatus, ColumnChange, GitError, PorcelainOptions, WorktreeChange,
        changes_affecting_paths, is_worktree_clean, parse_porcelain_v1z, worktree_changes,
    };

    /// Tracked state with only the index column set.
    const fn staged(change: ColumnChange) -> ChangeStatus {
        ChangeStatus::Tracked {
            index: Some(change),
            worktree: None,
        }
    }

    /// Tracked state with only the worktree column set.
    const fn unstaged(change: ColumnChange) -> ChangeStatus {
        ChangeStatus::Tracked {
            index: None,
            worktree: Some(change),
        }
    }

    #[test]
    fn parses_the_status_matrix() {
        let text = " M modified.rs\0?? new-file.txt\0A  staged-new.rs\0D  staged-del.rs\0 D unstaged-del.rs\0M  staged-mod.rs\0!! ignored.log\0";
        let changes = parse_porcelain_v1z(text).expect("the status-matrix fixture must parse");
        let got: Vec<(&str, ChangeStatus)> = changes
            .iter()
            .map(|c| (c.path.as_str(), c.status))
            .collect();
        assert_eq!(
            got,
            vec![
                ("modified.rs", unstaged(ColumnChange::Modified)),
                ("new-file.txt", ChangeStatus::Untracked),
                ("staged-new.rs", staged(ColumnChange::Added)),
                ("staged-del.rs", staged(ColumnChange::Deleted)),
                ("unstaged-del.rs", unstaged(ColumnChange::Deleted)),
                ("staged-mod.rs", staged(ColumnChange::Modified)),
                ("ignored.log", ChangeStatus::Ignored),
            ]
        );
    }

    #[test]
    fn both_columns_survive_mm() {
        // Porcelain XY is a matrix: staged AND unstaged modification on one
        // path must keep both halves.
        let changes = parse_porcelain_v1z("MM both.rs\0").expect("MM must parse");
        assert_eq!(
            changes.first().map(|c| c.status),
            Some(ChangeStatus::Tracked {
                index: Some(ColumnChange::Modified),
                worktree: Some(ColumnChange::Modified),
            })
        );
    }

    #[test]
    fn copies_parse_and_carry_their_source() {
        let changes =
            parse_porcelain_v1z("C  copy.rs\0original.rs\0").expect("a copy record must parse");
        assert_eq!(
            changes.first(),
            Some(&WorktreeChange {
                path: "copy.rs".to_owned(),
                status: staged(ColumnChange::Copied),
                old_path: Some("original.rs".to_owned()),
            })
        );
    }

    #[test]
    fn unmerged_paths_are_conflicted() {
        for fixture in [
            "UU theirs.rs\0",
            "AA ours.rs\0",
            "DD gone.rs\0",
            "AU mixed.rs\0",
        ] {
            let changes = parse_porcelain_v1z(fixture).expect("unmerged records must parse");
            assert_eq!(
                changes.first().map(|c| c.status),
                Some(ChangeStatus::Conflicted),
                "{fixture:?}"
            );
        }
    }

    #[test]
    fn rename_consumes_the_source_field() {
        let text = "R  new-name.rs\0old-name.rs\0?? other.txt\0";
        let changes = parse_porcelain_v1z(text).expect("the rename fixture must parse");
        assert_eq!(
            changes.first(),
            Some(&WorktreeChange {
                path: "new-name.rs".to_owned(),
                status: staged(ColumnChange::Renamed),
                old_path: Some("old-name.rs".to_owned()),
            })
        );
        assert_eq!(
            changes.get(1).map(|c| c.path.as_str()),
            Some("other.txt"),
            "the record after the rename source is parsed normally"
        );
    }

    #[test]
    fn unknown_code_and_short_record_are_parse_errors() {
        let unknown = parse_porcelain_v1z("ZZ weird.rs\0");
        assert!(
            matches!(unknown, Err(GitError::ParseError { .. })),
            "{unknown:?}"
        );
        let short = parse_porcelain_v1z("M\0");
        assert!(
            matches!(short, Err(GitError::ParseError { .. })),
            "{short:?}"
        );
    }

    #[test]
    fn path_filter_is_directory_boundary_not_substring() {
        let changes = vec![
            WorktreeChange {
                path: "specs/a.md".to_owned(),
                status: unstaged(ColumnChange::Modified),
                old_path: None,
            },
            WorktreeChange {
                path: "specs-other/b.md".to_owned(),
                status: unstaged(ColumnChange::Modified),
                old_path: None,
            },
            WorktreeChange {
                path: "README.md".to_owned(),
                status: unstaged(ColumnChange::Modified),
                old_path: None,
            },
        ];
        let hits = changes_affecting_paths(&changes, &["specs"]);
        assert_eq!(hits.len(), 1, "specs-other/ must NOT match: {hits:?}");
        assert_eq!(hits.first().map(|c| c.path.as_str()), Some("specs/a.md"));
        let exact = changes_affecting_paths(&changes, &["README.md"]);
        assert_eq!(exact.len(), 1, "exact file path matches");
    }

    #[test]
    fn rename_matches_via_old_path() {
        let changes = vec![WorktreeChange {
            path: "elsewhere/new.rs".to_owned(),
            status: staged(ColumnChange::Renamed),
            old_path: Some("specs/old.rs".to_owned()),
        }];
        let hits = changes_affecting_paths(&changes, &["specs"]);
        assert_eq!(
            hits.len(),
            1,
            "a rename out of a locked dir is a change to it"
        );
    }

    #[test]
    fn temp_repo_integration() {
        let dir = tempfile::tempdir().expect("fixture must create a temp dir");
        let root = dir.path();
        let git = |args: &[&str]| {
            let out = std::process::Command::new("git")
                .arg("-C")
                .arg(root)
                .args(args)
                .output()
                .expect("the git fixture command must run");
            assert!(out.status.success(), "git {args:?}: {:?}", out.stderr);
        };
        git(&["init", "-q"]);
        assert!(
            is_worktree_clean(root, PorcelainOptions::default())
                .expect("the clean query must succeed in a fresh repo"),
            "fresh repo is clean"
        );
        std::fs::write(root.join("a.txt"), "x").expect("the fixture must write the untracked file");
        let changes = worktree_changes(root, PorcelainOptions::default())
            .expect("the status query must succeed");
        assert_eq!(
            changes.first().map(|c| (c.path.as_str(), c.status)),
            Some(("a.txt", ChangeStatus::Untracked))
        );
        let without_untracked = worktree_changes(
            root,
            PorcelainOptions {
                include_untracked: false,
                ..PorcelainOptions::default()
            },
        )
        .expect("the status query must succeed");
        assert!(without_untracked.is_empty(), "untracked filtered out");
    }

    #[test]
    fn non_repo_is_not_a_repository() {
        let dir = tempfile::tempdir().expect("fixture must create a temp dir");
        let out = worktree_changes(dir.path(), PorcelainOptions::default());
        assert!(matches!(out, Err(GitError::NotARepository)), "{out:?}");
    }
}
