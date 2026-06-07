# aqc-filetree

Walk one filesystem directory root, return a **`FileTree`**. No Git commands, no file reads, no parsing, no product policy.

---

## Types

### `FileKind`

| Variant | Meaning |
|---------|---------|
| `File` | Regular file |
| `Directory` | Directory |
| `Symlink` | Symlink (only when `SymlinkPolicy::Record`) |

### `EntryOrigin`

| Variant | Meaning |
|---------|---------|
| `Primary` | Phase 1 walk |
| `Recovered` | Phase 2 walk (see [Recovery](#phase-2--recovery) below) |

### `FileEntry`

| Field | Type | Meaning |
|-------|------|---------|
| `rel_path` | `String` | Relative to walk root. `/`, UTF-8, no leading `/`, no `..` |
| `abs_path` | `PathBuf` | Absolute path |
| `kind` | `FileKind` | |
| `origin` | `EntryOrigin` | |

### `FileTree`

| Field | Type | Meaning |
|-------|------|---------|
| `root` | `PathBuf` | Walk root (`canonicalize` when possible) |
| `entries` | `Vec<FileEntry>` | Sorted by `rel_path` |

Queries: `entry`, `entries_with_origin`, `glob` (`globset` on `rel_path`).

---

### `SymlinkPolicy` (only symlink control)

| Variant | Traverse into symlink? | Appears in `FileTree`? |
|---------|------------------------|-------------------------|
| `Skip` | **Default.** No | No |
| `Record` | No | Yes, as `FileKind::Symlink` |
| `Follow` | Yes | Target tree as normal entries |

No separate `follow_symlinks` flag.

---

### `skip_dir_names` vs `.gitignore`

Different jobs; not a duplicate.

| Mechanism | What it does |
|-----------|----------------|
| **`respect_gitignore`** | Phase 1: paths ignored by `.gitignore` / `.ignore` are not listed as included |
| **`skip_dir_names`** | Never **descend** into a directory whose final path component matches (e.g. `target`, `node_modules`). Applies in **both** phases |

Why both: phase 2 walks **without** gitignore. Without `skip_dir_names`, the walker would descend into `target/` and scan every build artifact. Skip lists prune **directory descent** by known artifact folder names; gitignore prunes by VCS ignore rules. Overlap (e.g. `target/` often in both) is intentional, not redundant.

### `SkipDirPreset` (optional defaults module)

Constants only — caller merges into `skip_dir_names`. Not applied unless requested.

| Preset | Directory name components added |
|--------|----------------------------------|
| `Common` | `.git` |
| `Rust` | `target` |
| `Node` | `node_modules`, `dist` |
| `Python` | `__pycache__`, `.venv`, `venv`, `.pytest_cache`, `.mypy_cache`, `.tox` |
| `DotNet` | `bin`, `obj` |

```rust
// Example: WalkOptions { skip_dir_names: SkipDirPreset::merge(&[Common, Rust, Node]), .. }
```

`WalkOptions::default().skip_dir_names` = `SkipDirPreset::merge(&[Common, Rust, Node])` unless we prefer empty default and force explicit presets — **default: `merge([Common, Rust, Node])`** for practical walks.

---

### Phase 2 — recovery

#### Problem

Phase 1 with `respect_gitignore: true` does not list files that live under ignored trees (`target/`, `node_modules/`, etc.). Some tools still need **specific paths** inside those trees (e.g. a `Cargo.toml` or config under an ignored dir). Listing the entire ignored tree is wrong; listing **nothing** is also wrong.

#### What phase 2 does

1. Walk again **without** gitignore (still honors `skip_dir_names`, `skip_path_prefixes`, `max_depth`, `symlink_policy`).
2. Skip anything already found in phase 1.
3. If path matches caller’s **`RecoveryRules`** → add to tree with `EntryOrigin::Recovered`.

Phase 2 does **not** read file contents. It only adds path entries.

#### `RecoveryRules`

| Field | Type | Default | Meaning |
|-------|------|---------|---------|
| `exact_file_names` | `Vec<String>` | `[]` | Match file base name |
| `file_name_prefixes` | `Vec<String>` | `[]` | Match file base name prefix |
| `directory_names` | `Vec<String>` | `[]` | Match directory base name (presence sentinel) |
| `rel_path_suffixes` | `Vec<String>` | `[]` | Match full `rel_path` suffix |

Predicate: OR across fields. Product-specific filename lists live in **callers**, not in this crate.

`WalkOptions.recovery: Option<RecoveryRules>`, default **`None`** (phase 2 off).

---

### `WalkOptions`

| Field | Type | **Default** | Meaning |
|-------|------|-------------|---------|
| `respect_gitignore` | `bool` | `true` | Phase 1 gitignore-aware |
| `include_hidden` | `bool` | `true` | Dotfiles not skipped for being hidden |
| `symlink_policy` | `SymlinkPolicy` | `Skip` | |
| `skip_dir_names` | `Vec<String>` | `SkipDirPreset::merge(&[Common, Rust, Node])` | Do not descend into these dir name components |
| `skip_path_prefixes` | `Vec<String>` | `[]` | Do not enter these root-relative subtrees |
| `max_depth` | `Option<u32>` | `None` | `None` = no limit; `Some(n)` = max directory depth below root |
| `recovery` | `Option<RecoveryRules>` | `None` | Phase 2 off unless set |
| `glob_case_sensitive` | `bool` | `true` | For `FileTree::glob` |

---

## Walk algorithm

`build_file_tree(root, options) -> Result<FileTree, WalkError>`

**Phase 1:** gitignore per `respect_gitignore`; apply skip dirs/prefixes, `max_depth`, `symlink_policy`; `origin = Primary`.

**Phase 2:** only if `recovery.is_some()`; no gitignore; same skip/max_depth/symlink; match `RecoveryRules`; `origin = Recovered`.

Sort by `rel_path`, return `FileTree`.

---

## Non-goals

### Git porcelain / dirty worktree

**Not this crate.** “Porcelain” = stable, script-friendly `git status` output (`--porcelain`). A **dirty worktree** means Git sees unstaged/untracked/committed changes vs the index or `HEAD`.

Spec3 lock/verify may need to fail when locked spec paths changed on disk — that checks **Git’s view of change**, not “what paths exist.” `aqc-filetree` only answers existence/layout on disk at walk time. A separate **`aqc-git-worktree`** (or product code) would run `git status --porcelain` (or equivalent); it must not be folded into the file tree walk.

---

## Implementation backlog

- `rel_path_contains` rule shape — add only if suffix/prefix/name lists are insufficient

---

## AMENDMENTS (2026-06-07, post-build review)

- **`glob_case_sensitive` is DELETED from `WalkOptions`**: the glob query
  lives on the returned `FileTree`, which does not carry the walk options;
  case sensitivity is the query's parameter (`glob(pattern, case_sensitive)`).
- **Declared policies** (were silent implementation choices):
  non-UTF-8 file names are recorded lossily (`to_string_lossy`) in
  `rel_path`; a per-entry walk error (e.g. permission denied) aborts the walk
  with `WalkError::Io`; `glob` patterns let `*` match across `/`
  (`literal_separator(false)`).
