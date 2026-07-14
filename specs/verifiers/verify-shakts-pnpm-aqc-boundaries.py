#!/usr/bin/env python3
import json
import re
import subprocess
import sys
import tomllib
from pathlib import Path


RAW_SPEC = Path(sys.argv[1]).absolute()
SPEC = RAW_SPEC.resolve()
ENTRY = json.loads(SPEC.read_text())["requirements"]["custom"][int(sys.argv[3])]
ROOT = SPEC.parent.parent

NEW_PACKAGES = {
    "aqc-json-engine-core": ROOT / "packages/file-types/json/aqc-json-engine-core",
    "aqc-package-json-engine": ROOT / "packages/file-types/json/aqc-package-json-engine",
    "aqc-yaml-engine-core": ROOT / "packages/file-types/yaml/aqc-yaml-engine-core",
    "aqc-pnpm-workspace-yaml-engine": ROOT / "packages/file-types/yaml/aqc-pnpm-workspace-yaml-engine",
}
EXISTING_PACKAGES = {
    "aqc-text-file-engine": ROOT / "packages/file-types/text/aqc-text-file-engine",
    "aqc-toml-engine-core": ROOT / "packages/file-types/toml/aqc-toml-engine-core",
    "aqc-cargo-toml-engine": ROOT / "packages/file-types/toml/aqc-cargo-toml-engine",
    "aqc-clippy-toml-engine": ROOT / "packages/file-types/toml/aqc-clippy-toml-engine",
    "aqc-deny-toml-engine": ROOT / "packages/file-types/toml/aqc-deny-toml-engine",
    "aqc-rust-toolchain-toml-engine": ROOT / "packages/file-types/toml/aqc-rust-toolchain-toml-engine",
    "aqc-rustfmt-toml-engine": ROOT / "packages/file-types/toml/aqc-rustfmt-toml-engine",
}

APPROVED_CHANGED_FILES = {
    ".gitignore",
    ".github/workflows/release.yml",
    "fixture3.yaml",
    "fixtures/scripts/fixture3-shakts-pnpm-aqc.py",
    "release-plz.toml",
    "packages/aqc-file-engine-core/src/finding.rs",
    "packages/aqc-file-engine-core/src/lib.rs",
    "packages/aqc-file-engine-core/src/merge/mod.rs",
    "packages/aqc-file-engine-core/src/merge/model.rs",
    "packages/aqc-file-engine-core/src/merge/scalar.rs",
    "packages/aqc-file-engine-core/src/merge/scalar_assertion.rs",
    "packages/aqc-file-engine-core/src/merge/items.rs",
    "packages/aqc-file-engine-core/src/merge/lists.rs",
    "packages/aqc-file-engine-core/tests/architecture.rs",
    "packages/aqc-file-engine-core/tests/public_contract.rs",
    "packages/aqc-file-engine-core/tests/exact_items.rs",
    "packages/aqc-file-engine-core/tests/scalar_assertion.rs",
    "packages/file-types/toml/aqc-cargo-toml-engine/src/reconcile/dependencies/removals.rs",
    "packages/file-types/toml/aqc-cargo-toml-engine/src/reconcile/dependencies/required.rs",
    "packages/file-types/toml/aqc-cargo-toml-engine/src/reconcile/features.rs",
    "packages/file-types/toml/aqc-cargo-toml-engine/src/reconcile/lints.rs",
    "packages/file-types/toml/aqc-cargo-toml-engine/src/reconcile/package_lint_tables.rs",
    "packages/file-types/toml/aqc-clippy-toml-engine/src/reconcile/disallowed.rs",
    "packages/file-types/toml/aqc-clippy-toml-engine/src/reconcile/msrv.rs",
    "packages/file-types/toml/aqc-clippy-toml-engine/src/reconcile/thresholds.rs",
    "packages/file-types/toml/aqc-clippy-toml-engine/tests/merge.rs",
    "packages/file-types/toml/aqc-rust-toolchain-toml-engine/src/reconcile/settings_support.rs",
    "packages/file-types/toml/aqc-rustfmt-toml-engine/src/reconcile/settings/exact.rs",
    "packages/file-types/toml/aqc-rustfmt-toml-engine/src/reconcile/settings/ignore.rs",
    "packages/file-types/toml/aqc-toml-engine-core/src/finding.rs",
    "packages/file-types/toml/aqc-toml-engine-core/src/scalars.rs",
    "packages/file-types/toml/aqc-toml-engine-core/tests/core_contract.rs",
}

APPROVED_CHANGED_ROOTS = (
    ".worklogs/",
    "fixtures/shakts-pnpm-aqc/",
    "fixtures/approved/shakts-pnpm-aqc/",
    "fixtures/probes/shakts-pnpm-aqc/",
    "packages/file-types/json/aqc-json-engine-core/",
    "packages/file-types/json/aqc-package-json-engine/",
    "packages/file-types/text/aqc-text-engine-core/",
    "packages/file-types/text/aqc-text-file-engine/",
    "packages/file-types/yaml/aqc-yaml-engine-core/",
    "packages/file-types/yaml/aqc-pnpm-workspace-yaml-engine/",
    "specs/",
)


def changed_paths() -> tuple[set[str], list[str]]:
    changed: set[str] = set()
    errors: list[str] = []
    for command in (
        ["git", "diff", "--name-only", "HEAD"],
        ["git", "ls-files", "--others", "--exclude-standard"],
    ):
        result = subprocess.run(command, cwd=ROOT, capture_output=True, text=True, check=False)
        if result.returncode != 0:
            errors.append(f"cannot inspect repository diff: {result.stderr.strip()}")
        else:
            changed.update(result.stdout.splitlines())
    return changed, errors


def approved_changed_path(path: str) -> bool:
    if path in APPROVED_CHANGED_FILES or path.startswith(APPROVED_CHANGED_ROOTS):
        return True
    if path.endswith(("/Cargo.toml", "/Cargo.lock", "/deny.toml")) and path.startswith("packages/"):
        return True
    return False


def emit(errors: list[str]) -> None:
    evidence = {"check": ENTRY["check"], "status": "fail" if errors else "pass"}
    if errors:
        evidence["message"] = "; ".join(errors)
    print(json.dumps(evidence))


def manifest_for(directory: Path) -> dict | None:
    path = directory / "Cargo.toml"
    if not path.is_file():
        return None
    try:
        return tomllib.loads(path.read_text())
    except tomllib.TOMLDecodeError:
        return None


def dependencies(manifest: dict) -> dict[str, object]:
    result: dict[str, object] = {}
    for section in ("dependencies", "dev-dependencies", "build-dependencies"):
        result.update(manifest.get(section, {}))
    for target in manifest.get("target", {}).values():
        if isinstance(target, dict):
            for section in ("dependencies", "dev-dependencies", "build-dependencies"):
                result.update(target.get(section, {}))
    return result


def dependency_identity(local: str, value: object) -> str:
    if isinstance(value, dict) and isinstance(value.get("package"), str):
        return value["package"]
    return local


def source(directory: Path) -> str:
    source_dir = directory / "src"
    if not source_dir.is_dir():
        return ""
    paths = sorted(source_dir.glob("**/*.rs"))
    if (directory / "build.rs").is_file():
        paths.append(directory / "build.rs")
    return "\n".join(path.read_text() for path in paths)


def production_dependencies(manifest: dict) -> dict[str, object]:
    result = dict(manifest.get("dependencies", {}))
    for target in manifest.get("target", {}).values():
        if isinstance(target, dict):
            result.update(target.get("dependencies", {}))
    return result


def rust_source_and_tests(directory: Path) -> str:
    return "\n".join(
        path.read_text(errors="replace")
        for path in sorted(directory.glob("**/*.rs"))
        if "target" not in path.parts
    )


def check_layers(errors: list[str]) -> None:
    forbidden_io = [
        r"\bstd::fs\b",
        r"\bstd::env\b",
        r"\bstd::process\b",
        r"\bstd::net\b",
        r"\btokio::fs\b",
        r"\breqwest::",
    ]
    universal_definitions = [
        "ScalarAssertion",
        "ListRequirements",
        "ItemRequirements",
        "ForbiddenGlobRequirements",
        "KeyedItem",
        "Provenance",
        "ConflictEntry",
        "ResolvedRequirement",
        "ResolvedItemRequirements",
        "ResolvedListRequirements",
        "ResolvedForbiddenGlobRequirements",
        "EngineRequirement",
        "EngineOutput",
        "Finding",
    ]
    allowed_production_dependencies = {
        "aqc-json-engine-core": {"aqc-file-engine-core", "serde", "serde_json"},
        "aqc-package-json-engine": {
            "aqc-file-engine-core", "aqc-json-engine-core", "schemars", "serde"
        },
        "aqc-yaml-engine-core": {"aqc-file-engine-core", "yaml-edit"},
        "aqc-pnpm-workspace-yaml-engine": {
            "aqc-file-engine-core", "aqc-yaml-engine-core", "globset", "schemars", "serde"
        },
    }
    for name, directory in {**NEW_PACKAGES, **EXISTING_PACKAGES}.items():
        manifest = manifest_for(directory)
        if manifest is None:
            errors.append(f"{name}: valid manifest is missing")
            continue
        deps = {dependency_identity(local, value): value for local, value in dependencies(manifest).items()}
        if name in allowed_production_dependencies:
            production = {
                dependency_identity(local, value)
                for local, value in production_dependencies(manifest).items()
            }
            if production != allowed_production_dependencies[name]:
                errors.append(
                    f"{name}: production dependencies differ from pure-layer allowlist: {sorted(production)}"
                )
        for dep, value in deps.items():
            if isinstance(value, dict) and "path" in value:
                errors.append(f"{name}: path dependency {dep}")
            if dep.startswith(("shackles-", "shakrs-", "shakts-")):
                errors.append(f"{name}: upward Shackles dependency {dep}")
        if name.endswith("-engine-core"):
            sideways = [dep for dep in deps if dep.endswith("-engine") and dep != "aqc-file-engine-core"]
            if sideways:
                errors.append(f"{name}: format core depends on file engines {sorted(sideways)}")
        elif name.endswith("-engine"):
            allowed_core = {
                "aqc-package-json-engine": "aqc-json-engine-core",
                "aqc-pnpm-workspace-yaml-engine": "aqc-yaml-engine-core",
                "aqc-text-file-engine": "aqc-file-engine-core",
                "aqc-cargo-toml-engine": "aqc-toml-engine-core",
                "aqc-clippy-toml-engine": "aqc-toml-engine-core",
                "aqc-deny-toml-engine": "aqc-toml-engine-core",
                "aqc-rust-toolchain-toml-engine": "aqc-toml-engine-core",
                "aqc-rustfmt-toml-engine": "aqc-toml-engine-core",
            }[name]
            sideways = [
                dep for dep in deps
                if dep.startswith("aqc-") and dep.endswith("-engine") and dep not in {"aqc-file-engine-core", allowed_core}
            ]
            if sideways:
                errors.append(f"{name}: file engine depends sideways on {sorted(sideways)}")
        package_source = source(directory)
        if not package_source:
            errors.append(f"{name}: source is missing")
            continue
        for expression in forbidden_io:
            if re.search(expression, package_source):
                errors.append(f"{name}: forbidden IO/path API matches {expression}")
        for type_name in universal_definitions:
            if re.search(rf"\b(?:struct|enum|type)\s+{type_name}\b", package_source):
                errors.append(f"{name}: duplicates universal core type {type_name}")
        if re.search(r"#\s*\[\s*macro_export\s*\]|\bpub\s+use\s+[^;]*\*\s*;|\bpub\s+extern\s+crate\b", package_source, re.S):
            errors.append(f"{name}: public surface can bypass the exact facade inventory")
        deny = (directory / "deny.toml").read_text(errors="replace") if (directory / "deny.toml").is_file() else ""
        sibling_forbidden = {
            "aqc-json-engine-core": "aqc-yaml-engine-core",
            "aqc-package-json-engine": "aqc-yaml-engine-core",
            "aqc-yaml-engine-core": "aqc-json-engine-core",
            "aqc-pnpm-workspace-yaml-engine": "aqc-json-engine-core",
        }.get(name)
        if sibling_forbidden and sibling_forbidden not in deny:
            errors.append(f"{name}: deny.toml omits sibling format boundary {sibling_forbidden}")
        if name == "aqc-package-json-engine" and '"package.json"' in package_source:
            errors.append("aqc-package-json-engine: runtime embeds a deployment filename")
        if name == "aqc-pnpm-workspace-yaml-engine" and '"pnpm-workspace.yaml"' in package_source:
            errors.append("aqc-pnpm-workspace-yaml-engine: runtime embeds a deployment filename")
        complete_rust = rust_source_and_tests(directory)
        for downstream_name in ("shakts", "shakrs", "shackles"):
            if re.search(rf"(?i)\b{downstream_name}(?:[-_][a-z0-9_-]+)?\b", complete_rust):
                errors.append(f"{name}: source or tests name downstream product {downstream_name}")


def check_inventory(errors: list[str]) -> None:
    expected = {**NEW_PACKAGES, **EXISTING_PACKAGES}
    required_files = ["Cargo.toml", "Cargo.lock", "deny.toml", "src/lib.rs"]
    for name, directory in expected.items():
        manifest = manifest_for(directory)
        for relative in required_files:
            if not (directory / relative).is_file():
                errors.append(f"{name}: missing {relative}")
        if manifest is None:
            continue
        package = manifest.get("package", {})
        if package.get("name") != name:
            errors.append(f"{name}: manifest package name is {package.get('name')}")
        if package.get("publish") is not True:
            errors.append(f"{name}: package is not publishable")
        if manifest.get("workspace", {}).get("resolver") != "3":
            errors.append(f"{name}: not an independent resolver-3 workspace")
        if any(isinstance(value, dict) and "path" in value for value in dependencies(manifest).values()):
            errors.append(f"{name}: path dependency present")
    release = ROOT / "release-plz.toml"
    if not release.is_file():
        errors.append("release-plz.toml is missing")
    else:
        release_doc = tomllib.loads(release.read_text())
        released = {package.get("name") for package in release_doc.get("package", [])}
        missing = set(expected) - released
        if missing:
            errors.append(f"release inventory missing {sorted(missing)}")
    old = ROOT / "packages/file-types/text/aqc-text-engine-core"
    if old.exists():
        errors.append("obsolete aqc-text-engine-core package still exists")
    for relative in (
        "fixture3.yaml",
        "fixtures/shakts-pnpm-aqc/contracts.json",
        "fixtures/scripts/fixture3-shakts-pnpm-aqc.py",
        "fixtures/probes/shakts-pnpm-aqc/Cargo.toml",
        "fixtures/probes/shakts-pnpm-aqc/Cargo.lock",
        "fixtures/probes/shakts-pnpm-aqc/src/main.rs",
        "fixtures/approved/shakts-pnpm-aqc/approved.normalized.json",
    ):
        if not (ROOT / relative).is_file():
            errors.append(f"AQC Fixture3 artifact missing: {relative}")
    if not any("pnpm" in path.name.lower() for path in (ROOT / ".worklogs").glob("*.md")):
        errors.append("AQC pnpm implementation worklog is missing")
    inventory_files = [ROOT / "release-plz.toml"]
    for support_root in (ROOT / "scripts", ROOT / ".githooks"):
        if support_root.is_dir():
            inventory_files.extend(path for path in support_root.glob("**/*") if path.is_file())
    for directory in expected.values():
        if directory.is_dir():
            inventory_files.extend(
                path for path in directory.glob("**/*")
                if path.is_file() and "target" not in path.parts
            )
    tracked_text = "\n".join(
        path.read_text(errors="replace") for path in sorted(set(inventory_files))
        if path.suffix in {".toml", ".rs", ".md", ".json", ".yaml", ".yml", ".sh", ".py"}
    )
    if "aqc-text-engine-core" in tracked_text:
        errors.append("repository support or source files retain aqc-text-engine-core references")
    workflow = ROOT / ".github/workflows/release.yml"
    if not workflow.is_file():
        errors.append("release workflow is missing")
    else:
        workflow_text = workflow.read_text()
        for directory in expected.values():
            manifest = str((directory / "Cargo.toml").relative_to(ROOT))
            if manifest not in workflow_text:
                errors.append(f"release workflow omits {manifest}")
        for marker in (
            "release-foundations:",
            "release-format-cores:",
            "needs: release-foundations",
            "release-file-engines:",
            "needs: release-format-cores",
            "[patch.crates-io]",
        ):
            if marker not in workflow_text:
                errors.append(f"release workflow omits ordered-release marker {marker}")


def mismatch_bodies(source_text: str) -> list[str]:
    bodies = []
    offset = 0
    while (index := source_text.find("Finding::Mismatch", offset)) >= 0:
        brace = source_text.find("{", index)
        if brace < 0:
            break
        depth = 1
        for end in range(brace + 1, len(source_text)):
            if source_text[end] == "{":
                depth += 1
            elif source_text[end] == "}":
                depth -= 1
                if depth == 0:
                    bodies.append(source_text[brace + 1:end])
                    offset = end + 1
                    break
        else:
            break
    return bodies


def check_existing_limit(errors: list[str]) -> None:
    forbidden_aliases = ["aqc_text_engine_core", "AqcTextEngineCore", "TextEngineCore"]
    for name, directory in EXISTING_PACKAGES.items():
        package_source = source(directory)
        if not package_source:
            errors.append(f"{name}: source is missing")
            continue
        for body in mismatch_bodies(package_source):
            if "key:" in body and "selector: None" not in body:
                errors.append(f"{name}: existing mismatch does not supply selector: None")
                break
        if "selector: Some(" in package_source:
            errors.append(f"{name}: existing runtime introduces an item selector")
        for alias in forbidden_aliases:
            if alias in package_source:
                errors.append(f"{name}: compatibility alias/reference {alias}")
        manifest = manifest_for(directory)
        if manifest:
            for dep, value in dependencies(manifest).items():
                if isinstance(value, dict) and "path" in value:
                    errors.append(f"{name}: path dependency {dep}")
    for directory in NEW_PACKAGES.values():
        package_source = source(directory)
        if re.search(r"(?i)deprecated|compat(?:ibility)?\s+(?:alias|shim)", package_source):
            errors.append(f"{directory.relative_to(ROOT)}: deprecated compatibility API introduced")
    changed, change_errors = changed_paths()
    errors.extend(change_errors)
    unexpected = sorted(path for path in changed if not approved_changed_path(path))
    errors.extend(f"unapproved repository change: {path}" for path in unexpected)
    errors.extend(
        f"generated artifact is forbidden: {path}"
        for path in sorted(changed)
        if "__pycache__" in Path(path).parts or path.endswith(".pyc")
    )


def symlink_errors() -> list[str]:
    errors: list[str] = []
    required = [RAW_SPEC, Path(__file__).absolute(), ROOT / "specs/verifiers", ROOT / ".github/workflows/release.yml"]
    required.extend(NEW_PACKAGES.values())
    required.extend(EXISTING_PACKAGES.values())
    for path in required:
        current = path
        while current != ROOT.parent:
            if current.is_symlink():
                errors.append(f"required artifact uses symlink: {current.relative_to(ROOT)}")
                break
            if current == ROOT:
                break
            current = current.parent
    return errors


errors: list[str] = []
errors.extend(symlink_errors())
check = ENTRY["check"]
if check == "layer-and-purity-boundaries":
    check_layers(errors)
elif check == "workspace-inventory-gates-and-release-records":
    check_inventory(errors)
elif check == "existing-runtime-change-limit":
    check_existing_limit(errors)
else:
    errors.append(f"unsupported boundary check {check}")
emit(errors)
