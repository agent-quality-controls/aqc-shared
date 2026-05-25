# aqc-git-helpers

Read-only Git **worktree state** for Spec3 lock/verify. Not a general Git library.

Runs `git` as a subprocess, parses **porcelain** output, normalises paths like Spec3 / `aqc-filetree`. No commits, merges, or object database access in V1.

Primary consumer: **Spec3** (`lock`, `status`, `verify`). Guardrail3 hooks may keep using shell; adopt this only when Rust needs the same checks.

---

## Questions this crate answers

| Question | Spec3 use |
|----------|-----------|
| Is this directory a Git repo? | Before lock/verify |
| Is the worktree clean? | `lock` must fail if dirty |
| Which repo-relative paths changed? | `status`, `verify` drift |
| Did any **locked** path change? | `verify` after lock |

Disk existence is **`aqc-filetree`**. Git change detection is **`aqc-git-helpers`**. Different inputs.

---

## Git invocation

| Constant | Value |
|----------|-------|
| `PORCELAIN_VERSION` | `v1` |
| `STATUS_ARGS` | `git status --porcelain=v1 -z` (run with `-C <repo_root>`) |

Parse NUL-separated records. No porcelain v2 in V1.

Non-Git directories: `GitError::NotARepository` (detect via `git rev-parse --is-inside-work-tree` or failed status).

---

## Types

### `ChangeStatus`

| Variant | Porcelain prefix (v1) |
|---------|------------------------|
| `StagedNew` | `A` |
| `StagedModified` | `M` (index column) |
| `StagedDeleted` | `D` (index) |
| `StagedRenamed` | `R` (index) |
| `UnstagedModified` | ` M` |
| `UnstagedDeleted` | ` D` |
| `UnstagedRenamed` | ` R` (work tree) |
| `Untracked` | `??` |
| `Ignored` | `!!` (only if porcelain lists ignored; optional filter) |

Exact mapping table lives in implementation; tests use fixture byte strings.

### `WorktreeChange`

| Field | Type | Meaning |
|-------|------|---------|
| `path` | `String` | Repo-relative, `/`, UTF-8 (after normalise) |
| `status` | `ChangeStatus` | |
| `old_path` | `Option<String>` | Rename source path when applicable |

### `PorcelainOptions`

| Field | Type | Default | Meaning |
|-------|------|---------|---------|
| `include_ignored` | `bool` | `false` | Keep `!!` entries in output |
| `include_untracked` | `bool` | `true` | Keep `??` entries |

---

## API (V1)

```rust
/// Run porcelain status at `repo_root` and return all changes.
pub fn worktree_changes(
    repo_root: impl AsRef<Path>,
    options: PorcelainOptions,
) -> Result<Vec<WorktreeChange>, GitError>;

/// True when `worktree_changes` is empty (respecting `PorcelainOptions`).
pub fn is_worktree_clean(
    repo_root: impl AsRef<Path>,
    options: PorcelainOptions,
) -> Result<bool, GitError>;

/// Subset of `changes` whose `path` (or `old_path` for renames) is in `paths`
/// or is under a directory in `paths` (directory = path with trailing semantics TBD in impl).
pub fn changes_affecting_paths(
    changes: &[WorktreeChange],
    paths: &[&str],
) -> Vec<WorktreeChange>;

/// Convenience: `worktree_changes` then `changes_affecting_paths`.
pub fn dirty_paths(
    repo_root: impl AsRef<Path>,
    paths: &[&str],
    options: PorcelainOptions,
) -> Result<Vec<WorktreeChange>, GitError>;
```

Path normalisation: reject `..`, normalise separators to `/`, match Spec3 spec path rules.

---

## `GitError`

| Variant | When |
|---------|------|
| `NotARepository` | Not inside a Git worktree |
| `GitNotInstalled` | `git` executable missing |
| `CommandFailed { command, stderr }` | Non-zero exit |
| `ParseError { message }` | Unparseable porcelain line |

---

## Non-goals

- `commit`, `push`, `fetch`, merge, rebase, checkout
- Reading blob content from Git objects
- Replacing hook shell scripts in Guardrail3
- Libgit2 / Gitoxide (V1 uses subprocess only)
- Shared finding / evidence types (stay in each product)

---

## Tests

- Unit tests on **fixture porcelain strings** (no real repo required for parser).
- Few integration tests in a temp `git init` repo (optional, local only).
