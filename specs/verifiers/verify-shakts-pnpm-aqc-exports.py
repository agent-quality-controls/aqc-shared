#!/usr/bin/env python3
import json
import re
import sys
import tomllib
from pathlib import Path


SPEC = Path(sys.argv[1]).resolve()
CATEGORY = sys.argv[2]
INDEX = int(sys.argv[3])
ROOT = SPEC.parent.parent
BLOCK = json.loads(SPEC.read_text())["requirements"][CATEGORY][INDEX]


def package_source(package: str) -> tuple[Path | None, str]:
    for manifest_path in sorted((ROOT / "packages").glob("**/Cargo.toml")):
        try:
            manifest = tomllib.loads(manifest_path.read_text())
        except (OSError, tomllib.TOMLDecodeError):
            continue
        if manifest.get("package", {}).get("name") == package:
            lib = manifest_path.parent / "src/lib.rs"
            return lib, lib.read_text() if lib.is_file() else ""
    return None, ""


def public_names(source: str) -> set[str]:
    names = set(
        re.findall(
            r"(?m)^\s*pub\s+(?:unsafe\s+)?(?:struct|enum|trait|type|const|static|fn|mod)\s+([A-Za-z_][A-Za-z0-9_]*)",
            source,
        )
    )
    for statement in re.findall(r"(?ms)^\s*pub\s+use\s+(.+?);", source):
        statement = re.sub(r"\bas\s+_\b", "", statement)
        aliases = re.findall(r"\bas\s+([A-Za-z_][A-Za-z0-9_]*)", statement)
        names.update(aliases)
        statement = re.sub(r"\bas\s+[A-Za-z_][A-Za-z0-9_]*", "", statement)
        if "{" in statement:
            for group in re.findall(r"\{([^{}]*)\}", statement, re.S):
                names.update(
                    name
                    for name in re.findall(r"\b[A-Za-z_][A-Za-z0-9_]*\b", group)
                    if name not in {"self", "super", "crate"}
                )
        else:
            tail = re.search(r"([A-Za-z_][A-Za-z0-9_]*)\s*$", statement.strip())
            if tail and tail.group(1) not in {"self", "super", "crate"}:
                names.add(tail.group(1))
    return names


def emit(item: str, ok: bool, message: str | None = None) -> None:
    evidence = {"item": item, "status": "pass" if ok else "fail"}
    if message:
        evidence["message"] = message
    print(json.dumps(evidence))


manifest, source = package_source(BLOCK["package"])
exports = public_names(source)
missing_package = manifest is None
if "exact" in BLOCK:
    exact = set(BLOCK["exact"])
    bypass = bool(re.search(r"#\s*\[\s*macro_export\s*\]|\bpub\s+use\s+[^;]*\*\s*;|\bpub\s+extern\s+crate\b", source, re.S))
    ok = not missing_package and not bypass and exports == exact
    message = None if ok else (
        f"package {BLOCK['package']} is missing"
        if missing_package
        else f"public exports differ or bypass inventory: missing={sorted(exact - exports)}, extra={sorted(exports - exact)}, bypass={bypass}"
    )
    emit("exact-public-facade", ok, message)
for item in BLOCK.get("required", []):
    ok = not missing_package and item in exports
    message = None if ok else (
        f"package {BLOCK['package']} is missing"
        if missing_package
        else f"{item} is not exported from {manifest.relative_to(ROOT)}"
    )
    emit(item, ok, message)
for item in BLOCK.get("forbidden", []):
    ok = not missing_package and item not in exports
    message = None if ok else (
        f"package {BLOCK['package']} is missing, so its export boundary cannot be verified"
        if missing_package
        else f"forbidden export {item} is public in {manifest.relative_to(ROOT)}"
    )
    emit(item, ok, message)
