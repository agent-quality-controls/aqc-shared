#!/usr/bin/env python3
import hashlib
import json
import re
import subprocess
import sys
import tomllib
from pathlib import Path


SPEC = Path(sys.argv[1]).resolve()
ENTRY = json.loads(SPEC.read_text())["requirements"]["custom"][int(sys.argv[3])]
ROOT = SPEC.parent.parent


def emit(ok: bool, message: str) -> None:
    result = {"check": ENTRY["check"], "status": "pass" if ok else "fail"}
    if message:
        result["message"] = message
    print(json.dumps(result))


def run(argv: list[str], timeout: int = 45) -> subprocess.CompletedProcess[str]:
    return subprocess.run(argv, cwd=ROOT, text=True, capture_output=True, timeout=timeout, check=False)


def balanced_body(source: str, declaration: str) -> str | None:
    match = re.search(declaration + r"[^\{]*\{", source, re.S)
    if not match:
        return None
    start = match.end()
    depth = 1
    for index in range(start, len(source)):
        if source[index] == "{":
            depth += 1
        elif source[index] == "}":
            depth -= 1
            if depth == 0:
                return source[start:index]
    return None


def balanced_bodies(source: str, declaration: str) -> list[str]:
    bodies = []
    offset = 0
    while match := re.search(declaration + r"[^\{]*\{", source[offset:], re.S):
        declaration_start = offset + match.start()
        body = balanced_body(source[declaration_start:], declaration)
        if body is None:
            break
        bodies.append(body)
        offset = declaration_start + match.end()
    return bodies


def split_fields(body: str) -> dict[str, tuple[str, str]]:
    body = re.sub(r"(?m)^\s*///.*$", "", body)
    fields: dict[str, tuple[str, str]] = {}
    start = 0
    depths = {"<": 0, "(": 0, "[": 0, "{": 0}
    pairs = {">": "<", ")": "(", "]": "[", "}": "{"}
    for index, char in enumerate(body + ","):
        if char in depths:
            depths[char] += 1
        elif char in pairs:
            depths[pairs[char]] -= 1
        elif char == "," and not any(depths.values()):
            part = body[start:index].strip()
            start = index + 1
            match = re.match(r"(pub(?:\(crate\))?)\s+([A-Za-z_][A-Za-z0-9_]*)\s*:\s*(.+)", part, re.S)
            if match:
                fields[match.group(2)] = (match.group(1), re.sub(r"\s+", "", match.group(3)))
    return fields


def merge_contracts() -> tuple[bool, str]:
    errors = []
    for relative, resolved in ENTRY["engines"]:
        source = (ROOT / relative).read_text()
        result_type = re.compile(
            rf"Result<{re.escape(resolved)},Vec<(?:aqc_file_engine_core::)?ConflictEntry>>"
        )
        normalized = re.sub(r"\s+", "", source)
        if not result_type.search(normalized):
            errors.append(f"{relative}: merge does not return Result<{resolved},Vec<ConflictEntry>>")
        for token in ("ifconflicts.is_empty()", "Ok(resolved)", "Err(conflicts)"):
            if token not in normalized:
                errors.append(f"{relative}: missing {token}")
    core = (ROOT / "packages/aqc-file-engine-core/src/engine.rs").read_text()
    normalized = re.sub(r"\s+", "", core)
    for token in (
        "Fn(Vec<(Provenance,Requirements)>)->Result<ResolvedRequirements,Vec<ConflictEntry>>",
        "matchmerge(typed)", "Ok(resolved)=>reconcile_one(current_bytes,&resolved)", "Err(conflicts)=>EngineOutput",
        "expected_bytes:current_bytes.unwrap_or_default().to_vec()",
        "iftyped.len()!=reqs.len()",
        "Finding::InternalError",
    ):
        if token not in normalized:
            errors.append(f"core dispatch missing {token}")
    if "iftyped.is_empty()" not in normalized or "findings:Vec::new()" not in normalized:
        errors.append("core empty-input behavior is not explicit")
    return not errors, "; ".join(errors)


def root_contracts() -> tuple[bool, str]:
    errors = []
    for relative, name, expected in ENTRY["roots"]:
        source = (ROOT / relative).read_text()
        body = balanced_body(source, rf"pub\s+struct\s+{re.escape(name)}\b")
        if body is None:
            errors.append(f"{relative}: missing {name}")
            continue
        fields = split_fields(body)
        if set(fields) != set(expected):
            errors.append(f"{name}: fields {sorted(fields)} != {sorted(expected)}")
            continue
        for field, (visibility, field_type) in fields.items():
            if visibility != "pub(crate)":
                errors.append(f"{name}.{field}: visibility is {visibility}")
            getter = re.compile(
                rf"#\[must_use\]\s*pub\s+(?:const\s+)?fn\s+{re.escape(field)}\s*\(\s*&self\s*,?\s*\)\s*->\s*(.*?)\s*\{{\s*(.*?)\s*\}}",
                re.S,
            ).search(source)
            if not getter:
                errors.append(f"{name}.{field}: missing immutable #[must_use] getter")
                continue
            body = re.sub(r"\s+", "", getter.group(2))
            if body not in {f"&self.{field}", f"self.{field}.as_ref()"}:
                errors.append(f"{name}.{field}: getter does not return a borrowed field view")
        prefix = source[: source.find(f"pub struct {name}")]
        derive = re.findall(r"#\[derive\(([^]]+)\)\]", prefix)[-1:]
        if not derive or not {"Clone", "Debug", "Default"}.issubset(set(re.findall(r"\w+", derive[0]))):
            errors.append(f"{name}: Clone, Debug, Default not retained")
        impls = "\n".join(balanced_bodies(source, rf"impl\s+{re.escape(name)}\b"))
        public_methods = set(re.findall(r"pub\s+(?:const\s+)?fn\s+(\w+)", impls))
        if public_methods != set(expected):
            errors.append(f"{name}: public methods {sorted(public_methods)} != getters {sorted(expected)}")
        if re.search(r"pub\s+fn\s+\w+\s*\([^)]*&mut\s+self|pub\s+fn\s+\w+[^-]*->\s*&mut", impls, re.S):
            errors.append(f"{name}: mutable public API exposed")
    return not errors, "; ".join(errors)


def dependency_contracts() -> tuple[bool, str]:
    records = {}
    errors = []
    for directory, package, old_version in ENTRY["packages"]:
        manifest_path = ROOT / directory / "Cargo.toml"
        manifest = tomllib.loads(manifest_path.read_text())
        version = manifest["package"]["version"]
        records[package] = (directory, version, manifest)
        old = tuple(map(int, old_version.split(".")))
        new = tuple(map(int, version.split(".")))
        if new[0] != 0 or new[1] <= old[1]:
            errors.append(f"{package}: {version} is not a new incompatible pre-1.0 generation after {old_version}")
        if manifest["package"].get("publish") is False:
            errors.append(f"{package}: publish is false")
    core_version = records["aqc-file-engine-core"][1]
    toml_version = records["aqc-toml-engine-core"][1]

    def permits_patch(declared: str | None, released: str) -> bool:
        if declared is None:
            return False
        requirement = tuple(map(int, declared.split(".")[:3]))
        version = tuple(map(int, released.split(".")[:3]))
        return requirement[:2] == version[:2] and requirement <= version

    for package, (directory, _version, manifest) in records.items():
        dependencies = manifest.get("dependencies", {})
        for local, value in dependencies.items():
            if isinstance(value, dict) and "path" in value:
                errors.append(f"{package}: path dependency {local}")
        if package != "aqc-file-engine-core":
            value = dependencies.get("aqc-file-engine-core")
            declared = value if isinstance(value, str) else (value or {}).get("version")
            if not permits_patch(declared, core_version):
                errors.append(f"{package}: file-engine core {declared} does not permit {core_version}")
            if package in {"aqc-cargo-toml-engine", "aqc-clippy-toml-engine"} and declared != core_version:
                errors.append(f"{package}: file-engine core minimum {declared} must equal API generation {core_version}")
        if package.endswith("-toml-engine") and package != "aqc-toml-engine-core":
            value = dependencies.get("aqc-toml-engine-core")
            declared = value if isinstance(value, str) else (value or {}).get("version")
            if not permits_patch(declared, toml_version):
                errors.append(f"{package}: TOML core {declared} does not permit {toml_version}")
        lock = tomllib.loads((ROOT / directory / "Cargo.lock").read_text())
        for dep_name, wanted in (("aqc-file-engine-core", core_version), ("aqc-toml-engine-core", toml_version)):
            if dep_name not in dependencies:
                continue
            matches = [p for p in lock["package"] if p["name"] == dep_name]
            if len(matches) != 1 or matches[0]["version"] != wanted or not matches[0].get("source", "").startswith("registry+"):
                errors.append(f"{package}: lock does not contain one registry {dep_name} {wanted}")
    return not errors, "; ".join(errors)


def changed_scope() -> tuple[bool, str]:
    committed = run(["git", "diff", "--name-only", ENTRY["baseline"], "HEAD", "--"])
    status = run(["git", "status", "--porcelain=v1", "-uall"])
    paths = committed.stdout.splitlines()
    for line in status.stdout.splitlines():
        path = line[3:]
        if " -> " in path:
            path = path.split(" -> ", 1)[1]
        paths.append(path)
    allowed_files = set(ENTRY["manifestOnly"] + ENTRY["supportFiles"])
    prefixes = ENTRY["runtimePrefixes"] + ENTRY["supportPrefixes"]
    bad = [path for path in paths if path not in allowed_files and not any(path.startswith(p) for p in prefixes)]
    return not bad, "out-of-scope changed paths: " + ", ".join(bad) if bad else ""


def frozen_digests() -> tuple[bool, str]:
    errors = []
    for root, count, expected in ENTRY["roots"]:
        tracked = run(["git", "ls-files", root]).stdout.splitlines()
        lines = [hashlib.sha256((ROOT / path).read_bytes()).hexdigest() + "  " + path for path in tracked]
        digest = hashlib.sha256(("\n".join(lines) + "\n").encode()).hexdigest()
        if len(tracked) != count or digest != expected:
            errors.append(f"{root}: files={len(tracked)} sha256={digest}")
    return not errors, "; ".join(errors)


def targeted_tests() -> tuple[bool, str]:
    errors = []
    for argv in ENTRY["commands"]:
        result = run(argv)
        if result.returncode:
            tail = "\n".join((result.stdout + result.stderr).splitlines()[-8:])
            errors.append(f"{' '.join(argv)} failed ({result.returncode}): {tail}")
    return not errors, "; ".join(errors)


CHECKS = {
    "merge-result-contracts": merge_contracts,
    "resolved-root-encapsulation": root_contracts,
    "coherent-registry-generations": dependency_contracts,
    "exact-changed-path-scope": changed_scope,
    "frozen-forbidden-source-digests": frozen_digests,
    "targeted-contract-tests": targeted_tests,
}

try:
    ok, message = CHECKS[ENTRY["check"]]()
    emit(ok, message)
except Exception as error:
    emit(False, f"verifier exception: {type(error).__name__}: {error}")
