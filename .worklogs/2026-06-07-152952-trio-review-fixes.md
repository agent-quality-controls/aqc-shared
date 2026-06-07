# Post-build review fixes: git-helpers matrix model + filetree declared policies

## Summary
Fixes from the package-by-package review of the trio (plans amended first):

- **aqc-git-helpers**: `ChangeStatus` remodeled as the porcelain MATRIX it is
  -- `Tracked { index: Option<ColumnChange>, worktree: Option<ColumnChange> }
  | Conflicted | Untracked | Ignored` with `ColumnChange` = Added | Modified |
  Deleted | Renamed | Copied | TypeChanged. Fixes: `MM` keeps both halves
  (was silently dropping the unstaged column); copies (`C`) parse and carry
  their `-z` source field (was a hard `ParseError` -- every query failed on a
  repo with status.renames=copies); typechanges (`T`) parse; unmerged combos
  are `Conflicted` (dirty, fail-safe). New tests: MM, copy+source, four
  unmerged shapes.
- **aqc-git-helpers robustness**: subprocess pins `LC_ALL=C`;
  `NotARepository` decided by `git rev-parse --is-inside-work-tree` after a
  failed status, never by matching localized stderr text.
- **aqc-filetree**: dead `WalkOptions.glob_case_sensitive` field DELETED (the
  query owns case sensitivity; the tree never carried the options). The three
  silent policies are now declared in docs + plan amendment: lossy non-UTF-8
  rel_paths, per-entry walk errors abort, glob `*` crosses `/`.

## Verification
clippy 0, all tests, fmt, dupes 6.9%, MSRV 1.85 check clean. Manifests
updated (ChangeStatus/ColumnChange closed sets, WalkOptions shape).
