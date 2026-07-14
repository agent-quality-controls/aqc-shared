#!/usr/bin/env python3
import json
import re
import sys
import tomllib
from pathlib import Path


SPEC = Path(sys.argv[1]).resolve()
ENTRY = json.loads(SPEC.read_text())["requirements"]["custom"][int(sys.argv[3])]
ROOT = SPEC.parent.parent

PACKAGES = [
    "packages/file-types/json/aqc-json-engine-core",
    "packages/file-types/json/aqc-package-json-engine",
    "packages/file-types/yaml/aqc-yaml-engine-core",
    "packages/file-types/yaml/aqc-pnpm-workspace-yaml-engine",
    "packages/file-types/text/aqc-text-file-engine",
    "packages/file-types/toml/aqc-toml-engine-core",
    "packages/file-types/toml/aqc-cargo-toml-engine",
    "packages/file-types/toml/aqc-clippy-toml-engine",
    "packages/file-types/toml/aqc-deny-toml-engine",
    "packages/file-types/toml/aqc-rust-toolchain-toml-engine",
    "packages/file-types/toml/aqc-rustfmt-toml-engine",
]

REGISTRY_SOURCE = "registry+https://github.com/rust-lang/crates.io-index"


def emit(errors: list[str]) -> None:
    evidence = {"check": ENTRY["check"], "status": "fail" if errors else "pass"}
    if errors:
        evidence["message"] = "; ".join(errors)
    print(json.dumps(evidence))


def dependency_version(value: object) -> str | None:
    if isinstance(value, str):
        return value
    if isinstance(value, dict):
        version = value.get("version")
        return version if isinstance(version, str) else None
    return None


def all_dependencies(manifest: dict) -> dict[str, object]:
    dependencies: dict[str, object] = {}
    for section in ("dependencies", "dev-dependencies", "build-dependencies"):
        dependencies.update(manifest.get(section, {}))
    for target in manifest.get("target", {}).values():
        if isinstance(target, dict):
            for section in ("dependencies", "dev-dependencies", "build-dependencies"):
                dependencies.update(target.get(section, {}))
    return dependencies


errors: list[str] = []
core_manifest_path = ROOT / "packages/aqc-file-engine-core/Cargo.toml"
if not core_manifest_path.is_file():
    errors.append("aqc-file-engine-core manifest is missing")
    core_version = None
else:
    core_manifest = tomllib.loads(core_manifest_path.read_text())
    core_version = core_manifest.get("package", {}).get("version")
    if core_version != "0.7.0":
        errors.append(f"aqc-file-engine-core version is {core_version}, expected 0.7.0")

for relative in PACKAGES:
    directory = ROOT / relative
    manifest_path = directory / "Cargo.toml"
    lock_path = directory / "Cargo.lock"
    if not manifest_path.is_file():
        errors.append(f"{relative}: manifest is missing")
        continue
    try:
        manifest = tomllib.loads(manifest_path.read_text())
    except tomllib.TOMLDecodeError as error:
        errors.append(f"{relative}: invalid manifest: {error}")
        continue
    package_version = manifest.get("package", {}).get("version")
    if package_version != "0.7.0":
        errors.append(f"{relative}: package version is {package_version}, expected 0.7.0")
    dependencies = all_dependencies(manifest)
    requirement = dependency_version(dependencies.get("aqc-file-engine-core"))
    if requirement != "0.7.0":
        errors.append(f"{relative}: aqc-file-engine-core requirement is {requirement}, expected 0.7.0")
    for name, value in dependencies.items():
        if isinstance(value, dict) and "path" in value:
            errors.append(f"{relative}: path dependency {name}")
    if manifest.get("package", {}).get("publish") is not True:
        errors.append(f"{relative}: package is not explicitly publishable")
    if not lock_path.is_file():
        errors.append(f"{relative}: lockfile is missing")
        continue
    try:
        lock = tomllib.loads(lock_path.read_text())
    except tomllib.TOMLDecodeError as error:
        errors.append(f"{relative}: invalid lockfile: {error}")
        continue
    lock_packages = lock.get("package", [])
    cores = [package for package in lock_packages if package.get("name") == "aqc-file-engine-core"]
    versions = sorted({package.get("version") for package in cores})
    if len(cores) != 1 or versions != ["0.7.0"]:
        errors.append(f"{relative}: lockfile core generations are {versions}, expected ['0.7.0']")
registry_tuples: dict[
    tuple[object, object], dict[tuple[object, object], list[str]]
] = {}
lock_paths = [
    path
    for path in (ROOT / "packages").glob("**/Cargo.lock")
    if "target" not in path.relative_to(ROOT).parts
]
for lock_path in sorted(lock_paths):
    relative_lock = str(lock_path.relative_to(ROOT))
    manifest_path = lock_path.with_name("Cargo.toml")
    if not manifest_path.is_file():
        errors.append(f"{relative_lock}: neighboring manifest is missing")
        continue
    try:
        manifest = tomllib.loads(manifest_path.read_text())
        lock = tomllib.loads(lock_path.read_text())
    except tomllib.TOMLDecodeError as error:
        errors.append(f"{relative_lock}: invalid manifest or lockfile: {error}")
        continue
    workspace_name = manifest.get("package", {}).get("name")
    occurrences: dict[tuple[object, object], int] = {}
    for package in lock.get("package", []):
        name = package.get("name")
        if not isinstance(name, str) or not name.startswith("aqc-") or name == workspace_name:
            continue
        version = package.get("version")
        package_version = (name, version)
        occurrences[package_version] = occurrences.get(package_version, 0) + 1
        if version != "0.7.0":
            errors.append(f"{relative_lock}: {name} is version {version}, expected 0.7.0")
        source = package.get("source")
        checksum = package.get("checksum")
        if source is None and checksum is None:
            continue
        if source != REGISTRY_SOURCE:
            errors.append(f"{relative_lock}: {name} {version} has unsupported source {source}")
        if not isinstance(checksum, str) or re.fullmatch(r"[0-9a-f]{64}", checksum) is None:
            errors.append(f"{relative_lock}: {name} {version} has an invalid registry checksum")
        else:
            registry_tuples.setdefault(package_version, {}).setdefault(
                (source, checksum), []
            ).append(relative_lock)
    for package_version, count in occurrences.items():
        if count != 1:
            errors.append(
                f"{relative_lock}: {package_version[0]} {package_version[1]} occurs {count} times"
            )

for package_version, tuples in sorted(registry_tuples.items(), key=lambda item: repr(item[0])):
    if len(tuples) == 1:
        continue
    rendered = ", ".join(
        f"{registry_tuple}: {sorted(relatives)}"
        for registry_tuple, relatives in sorted(tuples.items(), key=lambda item: repr(item[0]))
    )
    errors.append(
        f"{package_version[0]} {package_version[1]} registry source/checksum tuples differ "
        f"across lockfiles: {rendered}"
    )

core_lock_path = ROOT / "packages/aqc-file-engine-core/Cargo.lock"
if core_lock_path.is_file():
    core_lock = tomllib.loads(core_lock_path.read_text())
    own = [package for package in core_lock.get("package", []) if package.get("name") == "aqc-file-engine-core"]
    if len(own) != 1 or own[0].get("version") != "0.7.0":
        errors.append("aqc-file-engine-core lockfile does not identify package version 0.7.0")
else:
    errors.append("aqc-file-engine-core lockfile is missing")

emit(errors)
