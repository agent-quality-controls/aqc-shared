#!/usr/bin/env python3
import json
import re
import subprocess
import sys
import tomllib
from pathlib import Path


SPEC = Path(sys.argv[1]).resolve()
CATEGORY = sys.argv[2]
INDEX = int(sys.argv[3])
DATA = json.loads(SPEC.read_text())
ENTRY = DATA["requirements"][CATEGORY][INDEX]
ROOT = SPEC.parent.parent
PACKAGES = {
    "aqc-jsonc-engine-core": ROOT / "packages/file-types/jsonc/aqc-jsonc-engine-core",
    "aqc-tsconfig-json-engine": ROOT / "packages/file-types/jsonc/aqc-tsconfig-json-engine",
    "aqc-package-json-engine": ROOT / "packages/file-types/json/aqc-package-json-engine",
}


def source_tree(package: Path) -> str:
    root = package / "src"
    return "\n".join(path.read_text() for path in sorted(root.glob("**/*.rs"))) if root.is_dir() else ""


def facade_names(package: Path) -> set[str]:
    path = package / "src/lib.rs"
    if not path.is_file():
        return set()
    text = path.read_text()
    names = set(re.findall(r"\bpub\s+(?:const|static|struct|enum|trait|type|fn)\s+([A-Za-z_][A-Za-z0-9_]*)", text))
    for group in re.findall(r"pub\s+use\s+[^;]*?\{([^}]*)\}", text, re.S):
        for item in group.split(","):
            name = item.strip().split(" as ")[-1].split("::")[-1].strip()
            if name and name != "self":
                names.add(name)
    names.update(re.findall(r"pub\s+use\s+[^;{}]+::([A-Za-z_][A-Za-z0-9_]*)\s*;", text))
    return names


def emit_typed() -> None:
    names = facade_names(PACKAGES[ENTRY["package"]])
    for polarity in ("required", "exists", "forbidden"):
        for item in ENTRY.get(polarity, []):
            present = item in names
            passed = present if polarity != "forbidden" else not present
            evidence = {"item": item, "status": "pass" if passed else "fail"}
            if not passed:
                evidence["message"] = f"facade name {item!r} has wrong presence for {polarity}"
            print(json.dumps(evidence))


def exact_tree(package: Path, expected: set[str]) -> list[str]:
    if not package.is_dir():
        return [f"missing package {package.relative_to(ROOT)}"]
    found = {str(path.relative_to(package)) for path in package.rglob("*") if path.is_file() and "target" not in path.parts}
    return [f"{package.name} tree missing {sorted(expected - found)} extra {sorted(found - expected)}"] if found != expected else []


def dependencies(package: Path) -> set[str]:
    manifest = package / "Cargo.toml"
    if not manifest.is_file():
        return set()
    data = tomllib.loads(manifest.read_text())
    names: set[str] = set()
    for section in ("dependencies", "dev-dependencies", "build-dependencies"):
        for key, value in data.get(section, {}).items():
            names.add(value.get("package", key) if isinstance(value, dict) else key)
    return names


def changed_paths() -> list[str]:
    paths: set[str] = set()
    for command in (["git", "diff", "--name-only", "HEAD"], ["git", "diff", "--name-only", "--cached"], ["git", "ls-files", "--others", "--exclude-standard"]):
        result = subprocess.run(command, cwd=ROOT, check=False, capture_output=True, text=True)
        if result.returncode == 0:
            paths.update(line for line in result.stdout.splitlines() if line)
    return sorted(paths)


def custom_errors() -> list[str]:
    check = ENTRY["check"]
    jsonc = PACKAGES["aqc-jsonc-engine-core"]
    tsconfig = PACKAGES["aqc-tsconfig-json-engine"]
    package_json = PACKAGES["aqc-package-json-engine"]
    if check == "exact-new-package-trees":
        base = {"Cargo.toml", "Cargo.lock", "LICENSE", "README.md", "deny.toml", "guardrail3-rs.toml", "src/lib.rs"}
        return exact_tree(jsonc, base | {"src/runtime/mod.rs", "src/runtime/parse.rs", "src/runtime/scalar.rs", "src/types/mod.rs", "src/types/object.rs", "src/types/options.rs", "tests/core_contract.rs"}) + exact_tree(tsconfig, base | {"src/runtime/engine.rs", "src/runtime/merge.rs", "src/runtime/mod.rs", "src/runtime/reconcile.rs", "src/types/mod.rs", "src/types/model.rs", "tests/contract.rs", "tests/engine_requirement.rs"})
    if check == "exact-public-facades-and-api-shapes":
        expected = {
            jsonc: {"ConfigScalar", "Finding", "JsoncObject", "JsoncParseOptions", "Provenance", "ResolvedRequirement", "ScalarAssertion", "ScalarValue", "parse_object_or_report", "reconcile_scalar_assertion"},
            tsconfig: {"ConflictEntry", "ENGINE_ID", "Provenance", "ResolvedMap", "ResolvedRequirement", "ResolvedTsconfigJsonRequirements", "ScalarAssertion", "TsconfigBooleanCompilerOption", "TsconfigJsonEngine", "TsconfigJsonRequirements"},
            package_json: {"ConflictEntry", "DevEnginePackageManagerRequirements", "ENGINE_ID", "PackageJsonEngine", "PackageJsonRequirements", "PackageManagerOnFail", "Provenance", "ResolvedDevEnginePackageManagerRequirements", "ResolvedMap", "ResolvedPackageJsonRequirements", "ResolvedRequirement", "ScalarAssertion"},
        }
        errors = []
        for package, names in expected.items():
            actual = facade_names(package)
            if actual != names:
                errors.append(f"{package.name} facade expected {sorted(names)}, found {sorted(actual)}")
        jsonc_text = source_tree(jsonc)
        tsconfig_text = source_tree(tsconfig)
        package_text = source_tree(package_json)
        required = [
            (jsonc_text, "pub fn set_scalar(&mut self"),
            (jsonc_text, "pub fn remove_value(&mut self"),
            (jsonc_text, "pub fn scalar(&self"),
            (jsonc_text, "pub fn render(&self"),
            (jsonc_text, "document: &mut JsoncObject"),
            (tsconfig_text, "pub boolean_compiler_options:"),
            (tsconfig_text, "&ResolvedMap<TsconfigBooleanCompilerOption, ScalarAssertion<bool>>"),
            (tsconfig_text, "pub const fn file_key(self) -> &'static str"),
            (package_text, "pub scripts: BTreeMap<String, ScalarAssertion<String>>"),
            (package_text, "pub dev_dependencies: BTreeMap<String, ScalarAssertion<String>>"),
            (package_text, "&ResolvedMap<String, ScalarAssertion<String>>"),
        ]
        errors.extend(f"missing API fragment {fragment}" for text, fragment in required if fragment not in text)
        for text, forbidden in [(jsonc_text, "pub fn set_scalar(&self"), (jsonc_text, "pub fn remove_value(&self")]:
            if forbidden in text:
                errors.append(f"corrected mutable signature violated by {forbidden}")
        aliases = re.findall(r"\bpub\s+type\s+([A-Za-z_][A-Za-z0-9_]*)", jsonc_text + tsconfig_text + package_text)
        if aliases:
            errors.append(f"public aliases forbidden: {aliases}")
        return errors
    if check == "exact-dependency-boundaries":
        expected = {
            jsonc: {"aqc-file-engine-core", "jsonc-parser", "tree-sitter", "tree-sitter-javascript"},
            tsconfig: {"aqc-file-engine-core", "aqc-jsonc-engine-core", "schemars", "serde"},
        }
        errors = []
        for package, names in expected.items():
            actual = dependencies(package)
            if actual != names:
                errors.append(f"{package.name} dependencies expected {sorted(names)}, found {sorted(actual)}")
            if "path =" in (package / "Cargo.toml").read_text() if (package / "Cargo.toml").is_file() else False:
                errors.append(f"{package.name} has path dependency")
        for manifest in ROOT.glob("packages/**/Cargo.toml"):
            for name in dependencies(manifest.parent):
                if name.startswith(("shackles-", "shakrs-", "shakts-")):
                    errors.append(f"AQC package {manifest.parent.relative_to(ROOT)} depends on {name}")
        exempt = {jsonc / "deny.toml", tsconfig / "deny.toml"}
        for deny in ROOT.glob("packages/**/deny.toml"):
            if deny not in exempt and "aqc-jsonc-engine-core" not in deny.read_text():
                errors.append(f"{deny.relative_to(ROOT)} does not forbid aqc-jsonc-engine-core")
        return errors
    if check == "attribution-migration-contract":
        core = source_tree(ROOT / "packages/aqc-file-engine-core")
        errors = []
        if "pub fn attribution(&self) -> Vec<Provenance>" not in core:
            errors.append("ResolvedRequirement::attribution missing")
        if "pub fn resolved_map_attribution" not in core:
            errors.append("resolved_map_attribution missing")
        if "pub fn mismatch(" in core or "pub fn push_mismatch(" in core:
            errors.append("duplicate core mismatch API present")
        consumers = [
            "packages/file-types/json/aqc-json-engine-core",
            "packages/file-types/toml/aqc-toml-engine-core",
            "packages/file-types/toml/aqc-cargo-toml-engine",
            "packages/file-types/toml/aqc-clippy-toml-engine",
            "packages/file-types/toml/aqc-deny-toml-engine",
            "packages/file-types/toml/aqc-rust-toolchain-toml-engine",
            "packages/file-types/toml/aqc-rustfmt-toml-engine",
            "packages/file-types/yaml/aqc-yaml-engine-core",
            "packages/file-types/yaml/aqc-pnpm-workspace-yaml-engine",
            "packages/file-types/text/aqc-text-file-engine",
        ]
        for consumer in consumers:
            text = source_tree(ROOT / consumer)
            if ".attribution()" not in text:
                errors.append(f"{consumer} does not use core attribution")
        for core_path in [ROOT / "packages/file-types/json/aqc-json-engine-core", ROOT / "packages/file-types/toml/aqc-toml-engine-core"]:
            text = source_tree(core_path)
            if re.search(r"\bfn\s+(?:attribution|push_mismatch)\s*\(", text):
                errors.append(f"format helper remains in {core_path.name}")
        for removed in [
            ROOT / "packages/file-types/json/aqc-json-engine-core/src/runtime/finding.rs",
            ROOT / "packages/file-types/toml/aqc-toml-engine-core/src/finding.rs",
        ]:
            if removed.exists():
                errors.append(f"obsolete attribution helper remains at {removed.relative_to(ROOT)}")
        for engine_path in [package_json, tsconfig]:
            text = source_tree(engine_path)
            if "resolved_map_attribution" not in text:
                errors.append(f"{engine_path.name} does not use resolved_map_attribution")
            if re.search(r"\bfn\s+map_attribution\s*\(", text):
                errors.append(f"duplicate map attribution helper remains in {engine_path.name}")
        return errors
    if check == "jsonc-and-engine-purity-boundaries":
        errors = []
        for package in [jsonc, tsconfig]:
            text = source_tree(package)
            for token in ["std::fs", "std::path", "std::process", "std::env", "Command::new", "reqwest", "ureq", "shakts", "shackles"]:
                if token in text:
                    errors.append(f"{package.name} contains forbidden boundary token {token}")
        jsonc_text = source_tree(jsonc)
        if "options: JsoncParseOptions" not in jsonc_text:
            errors.append("JSONC parser does not accept caller-owned JsoncParseOptions")
        for token in ["tsconfig", "typescript", "compilerOptions", "Tsc"]:
            if token in jsonc_text:
                errors.append(f"JSONC core contains tool-specific token {token}")
        if "jsonc_parser" in (jsonc / "src/lib.rs").read_text() if (jsonc / "src/lib.rs").is_file() else False:
            errors.append("JSONC facade leaks parser dependency types")
        return errors
    if check == "fixture-and-downstream-contract":
        paths = [ROOT / "fixtures/shakts-tsc-aqc/contracts.json", ROOT / "fixtures/scripts/fixture3-shakts-tsc-aqc.py", ROOT / "fixtures/approved/shakts-tsc-aqc/approved.normalized.json", ROOT / "fixtures/probes/shakts-tsc-aqc/src/main.rs"]
        if any(not path.is_file() for path in paths):
            return [f"missing fixture/probe artifact {path.relative_to(ROOT)}" for path in paths if not path.is_file()]
        text = "\n".join(path.read_text() for path in paths)
        markers = ["comment", "trailing", "hex", "binary", "octal", "utf8-bom", "single", "duplicate", "malformed", "preserv", "insertionPreserved", "bomPreserved", "extendedNumbersPreserved", "compilerOptions", "conflict", "init", "attribution", "downstream"]
        errors = [f"fixture/probe contract missing marker {marker}" for marker in markers if marker.lower() not in text.lower()]
        result = subprocess.run(
            ["fixture3", "check", "--suite", "shakts-tsc-aqc"],
            cwd=ROOT,
            capture_output=True,
            text=True,
            check=False,
        )
        if result.returncode != 0:
            errors.append(f"fixture3 shakts-tsc-aqc failed: {result.stdout}{result.stderr}")
        return errors
    if check == "changed-package-scope":
        allowed = set(ENTRY["allowed_paths"])
        changed = set(changed_paths())
        return [f"changed path outside plan scope: {path}" for path in sorted(changed - allowed)] + [f"planned path is unchanged: {path}" for path in sorted(allowed - changed)]
    return [f"unknown check {check}"]


if CATEGORY == "exports":
    emit_typed()
else:
    errors = custom_errors()
    evidence = {"check": ENTRY["check"], "status": "fail" if errors else "pass"}
    if errors:
        evidence["message"] = "; ".join(errors)
    print(json.dumps(evidence))
