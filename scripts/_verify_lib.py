"""Shared helpers for verify-layer-*.sh scripts.

Each layer's script is a thin shell wrapper that invokes a corresponding
function here. All scripts assume they are run from the aqc-shared repo
root (the wrappers ensure cwd via `cd "$(dirname "$0")/.."`).
"""

from __future__ import annotations

import os
import re
import subprocess
import sys
import tomllib
from dataclasses import dataclass
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parent.parent
DEFAULT_MANIFEST = (
    REPO_ROOT.parent
    / "guardrail3"
    / ".plans"
    / "g3v2-architecture"
    / "2026-05-26-191126-clippy-vertical-slice.md.manifest.toml"
)


def manifest_path() -> Path:
    env = os.environ.get("MANIFEST")
    if env:
        return Path(env).resolve()
    return DEFAULT_MANIFEST.resolve()


def load_manifest() -> dict:
    p = manifest_path()
    if not p.exists():
        die(f"manifest not found at {p}; set $MANIFEST to override")
    with p.open("rb") as fh:
        return tomllib.load(fh)


@dataclass
class CheckResult:
    name: str
    passed: bool
    detail: str = ""


def print_pass(name: str) -> None:
    print(f"PASS  {name}")


def print_fail(name: str, detail: str) -> None:
    print(f"FAIL  {name}")
    if detail:
        for line in detail.rstrip().splitlines():
            print(f"      {line}")


def report(layer: str, results: list[CheckResult]) -> int:
    """Print results and return an exit code suitable for the layer script."""
    failed = [r for r in results if not r.passed]
    for r in results:
        if r.passed:
            print_pass(r.name)
        else:
            print_fail(r.name, r.detail)
    print()
    if failed:
        print(f"=== {layer} ===  FAIL  ({len(failed)} of {len(results)})")
        return 1
    print(f"=== {layer} ===  PASS  ({len(results)} checks)")
    return 0


def die(msg: str) -> None:
    print(f"verifier error: {msg}", file=sys.stderr)
    sys.exit(2)


# ---------------------------------------------------------------------------
# Source scanning helpers (regex-based, not AST). Good enough for first pass.
# Where rigor matters, swap for syn-based parsing later.
# ---------------------------------------------------------------------------


def crate_src_files(crate_manifest_path: str) -> list[Path]:
    """All .rs files under <crate>/src/."""
    src = REPO_ROOT / crate_manifest_path / "src"
    if not src.exists():
        return []
    return sorted(src.rglob("*.rs"))


def read_all_src(crate_manifest_path: str) -> str:
    """Concatenate every .rs file in the crate. Used for grep-style checks."""
    out = []
    for p in crate_src_files(crate_manifest_path):
        out.append(p.read_text())
    return "\n".join(out)


# Matches `pub struct Foo`, `pub enum Foo`, `pub trait Foo`, `pub fn foo`,
# `pub type Foo`, `pub union Foo` and variants with generics (`pub struct Foo<A>`).
PUBLIC_ITEM_RE_TMPL = (
    r"(^|\n)\s*pub(?:\([^)]*\))?\s+(struct|enum|trait|fn|type|union)\s+{name}\b"
)


def public_item_present(src: str, item_name: str) -> bool:
    pattern = PUBLIC_ITEM_RE_TMPL.format(name=re.escape(item_name))
    return re.search(pattern, src, re.MULTILINE) is not None


def find_enum_block(src: str, enum_name: str) -> str | None:
    """Return the body of `pub enum Name { ... }` if found.

    Handles nesting by counting braces. Returns the contents between the
    outermost braces (excluding the braces themselves).
    """
    decl = re.search(
        rf"\bpub(?:\([^)]*\))?\s+enum\s+{re.escape(enum_name)}\b[^{{]*{{",
        src,
    )
    if not decl:
        return None
    start = decl.end()
    depth = 1
    i = start
    while i < len(src) and depth > 0:
        c = src[i]
        if c == "{":
            depth += 1
        elif c == "}":
            depth -= 1
        i += 1
    if depth != 0:
        return None
    return src[start : i - 1]


def parse_enum_variants(body: str) -> set[str]:
    """Extract variant identifiers from an enum body.

    A variant is an identifier at the start of a top-level (within the
    enum block) line, optionally followed by `(...)`, `{...}`, `=`, or `,`.
    We strip nested groups so commas/identifiers inside variant payloads
    don't get counted.
    """
    # Strip nested () and {} so payloads don't confuse the split.
    stripped = _strip_nested(body, "(", ")")
    stripped = _strip_nested(stripped, "{", "}")
    # Remove attributes (#[...]) and doc/comment lines.
    cleaned_lines = []
    for line in stripped.splitlines():
        s = line.strip()
        if not s:
            continue
        if s.startswith("//") or s.startswith("#["):
            continue
        cleaned_lines.append(s)
    cleaned = "\n".join(cleaned_lines)
    variants: set[str] = set()
    for token in cleaned.split(","):
        t = token.strip()
        if not t:
            continue
        # Variant is the leading identifier.
        m = re.match(r"([A-Za-z_][A-Za-z0-9_]*)", t)
        if m:
            variants.add(m.group(1))
    return variants


def _strip_nested(s: str, open_c: str, close_c: str) -> str:
    out = []
    depth = 0
    for c in s:
        if c == open_c:
            depth += 1
            continue
        if c == close_c:
            if depth > 0:
                depth -= 1
            continue
        if depth == 0:
            out.append(c)
    return "".join(out)


def find_struct_block(src: str, struct_name: str) -> str | None:
    """Return body of `pub struct Name { ... }`. Tuple structs return ''.

    Handles generics, lifetime params, and where clauses by scanning ahead
    to the first `{` or `;`.
    """
    decl = re.search(
        rf"\bpub(?:\([^)]*\))?\s+struct\s+{re.escape(struct_name)}\b",
        src,
    )
    if not decl:
        return None
    i = decl.end()
    while i < len(src) and src[i] not in "{;":
        i += 1
    if i >= len(src):
        return None
    if src[i] == ";":
        return ""  # unit struct
    # find matching brace
    start = i + 1
    depth = 1
    j = start
    while j < len(src) and depth > 0:
        c = src[j]
        if c == "{":
            depth += 1
        elif c == "}":
            depth -= 1
        j += 1
    if depth != 0:
        return None
    return src[start : j - 1]


def parse_struct_fields(body: str) -> list[tuple[str, str]]:
    """Return [(field_name, field_type)] for a struct body.

    Type strings are normalized: whitespace collapsed, but generics and
    references preserved. Visibility prefixes (`pub`, `pub(crate)`) stripped.
    """
    # Strip nested groups so a tuple type like (A, B) doesn't split on its comma.
    # We need to keep angle brackets so generics survive — strip () but
    # keep <> by treating < and > as ordinary chars in the splitter, then
    # parse with bracket counting.
    fields: list[tuple[str, str]] = []
    for raw in _split_top_level_commas(body):
        line = raw.strip().rstrip(",").strip()
        if not line:
            continue
        if line.startswith("//") or line.startswith("#["):
            continue
        # remove visibility prefix
        line = re.sub(r"^pub(?:\([^)]*\))?\s+", "", line)
        m = re.match(r"([A-Za-z_][A-Za-z0-9_]*)\s*:\s*(.+)$", line, re.DOTALL)
        if not m:
            continue
        name = m.group(1)
        typ = _normalize_type(m.group(2))
        fields.append((name, typ))
    return fields


def _split_top_level_commas(body: str) -> list[str]:
    """Split `body` on top-level commas, ignoring commas nested in <>, [], (), {}."""
    out: list[str] = []
    buf: list[str] = []
    depth_a = depth_p = depth_s = depth_c = 0
    for c in body:
        if c == "<":
            depth_a += 1
        elif c == ">":
            depth_a -= 1
        elif c == "(":
            depth_p += 1
        elif c == ")":
            depth_p -= 1
        elif c == "[":
            depth_s += 1
        elif c == "]":
            depth_s -= 1
        elif c == "{":
            depth_c += 1
        elif c == "}":
            depth_c -= 1
        if c == "," and depth_a == depth_p == depth_s == depth_c == 0:
            out.append("".join(buf))
            buf = []
        else:
            buf.append(c)
    if buf:
        out.append("".join(buf))
    return out


def _normalize_type(t: str) -> str:
    # collapse whitespace
    t = re.sub(r"\s+", "", t)
    return t


# ---------------------------------------------------------------------------
# Cargo dependency introspection (uses cargo metadata, no external crates).
# ---------------------------------------------------------------------------


def crate_cargo_toml_exists(crate_manifest_path: str) -> bool:
    return (REPO_ROOT / crate_manifest_path / "Cargo.toml").exists()


def crate_src_dir_exists(crate_manifest_path: str) -> bool:
    return (REPO_ROOT / crate_manifest_path / "src").is_dir()


def cargo_dependencies(crate_manifest_path: str) -> list[str] | None:
    """Return the list of direct dependency names declared by the crate's
    Cargo.toml. Reads the manifest with tomllib; no cargo invocation needed.

    Returns None if the Cargo.toml doesn't exist (so callers can distinguish
    "crate missing" from "crate has zero deps"). Returns [] for a present
    Cargo.toml with no deps.
    """
    manifest = REPO_ROOT / crate_manifest_path / "Cargo.toml"
    if not manifest.exists():
        return None
    with manifest.open("rb") as fh:
        data = tomllib.load(fh)
    deps = set()
    for section in ("dependencies", "dev-dependencies", "build-dependencies"):
        deps.update(data.get(section, {}).keys())
    return sorted(deps)


def workspace_root_for(crate_manifest_path: str) -> Path | None:
    """Walk up from crate path to find the workspace root (the Cargo.toml
    containing `[workspace]`).

    Returns None if no workspace ancestor exists (e.g. the crate path
    itself does not exist). Callers should treat None as "cannot run
    cargo commands here yet" and skip or fail accordingly.
    """
    p = REPO_ROOT / crate_manifest_path
    if not p.exists():
        return None
    while p != p.parent:
        cargo = p / "Cargo.toml"
        if cargo.exists():
            with cargo.open("rb") as fh:
                data = tomllib.load(fh)
            if "workspace" in data:
                return p
        p = p.parent
    return None


# ---------------------------------------------------------------------------
# Command runner for verification_command rows.
# ---------------------------------------------------------------------------


def run_command(cmd: str, cwd: Path) -> tuple[int, str]:
    proc = subprocess.run(
        cmd, shell=True, cwd=cwd, capture_output=True, text=True
    )
    out = (proc.stdout or "") + (proc.stderr or "")
    return proc.returncode, out
