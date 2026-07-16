#!/usr/bin/env python3
from __future__ import annotations

import json
import os
import subprocess
import sys
import tempfile
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SHACKLES = ROOT.parent / "shackles"
SPEC = json.loads(Path(sys.argv[1]).read_text())
ENTRY = SPEC["requirements"][sys.argv[2]][int(sys.argv[3])]

if sys.argv[2] == "exports":
    package_roots = {
        "aqc-file-engine-core": ROOT / "packages/aqc-file-engine-core",
        "aqc-toml-engine-core": ROOT / "packages/file-types/toml/aqc-toml-engine-core",
    }
    facade = (package_roots[ENTRY["package"]] / "src/lib.rs").read_text()
    for item in ENTRY.get("required", []):
        print(json.dumps({"item": item, "status": "pass" if item in facade else "fail"}))
    for item in ENTRY.get("exists", []):
        print(json.dumps({"item": item, "status": "pass" if item in facade else "fail"}))
    for item in ENTRY.get("forbidden", []):
        print(json.dumps({"item": item, "status": "pass" if item not in facade else "fail"}))
    raise SystemExit(0)

CHECK = ENTRY["check"]

WORKSPACES = [ROOT / ENTRY["workspace"]] if "workspace" in ENTRY else []
CORE = ROOT / "packages/aqc-file-engine-core"
JSON_ENGINE = ROOT / "packages/file-types/json/aqc-json-file-engine"
TOML_CORE = ROOT / "packages/file-types/toml/aqc-toml-engine-core"
YAML_ENGINE = ROOT / "packages/file-types/yaml/aqc-pnpm-workspace-yaml-engine"
DEPENDENCY_ONLY_LOCKS = {
    "packages/file-types/json/aqc-json-engine-core/Cargo.lock",
    "packages/file-types/json/aqc-package-json-engine/Cargo.lock",
    "packages/file-types/jsonc/aqc-tsconfig-json-engine/Cargo.lock",
    "packages/file-types/text/aqc-text-file-engine/Cargo.lock",
    "packages/file-types/toml/aqc-clippy-toml-engine/Cargo.lock",
    "packages/file-types/yaml/aqc-yaml-engine-core/Cargo.lock",
}


def source(root: Path) -> str:
    return "\n".join(
        path.read_text()
        for path in sorted(root.rglob("*.rs"))
        if "target" not in path.relative_to(root).parts
        and any(part in {"src", "tests"} for part in path.relative_to(root).parts)
    )


def require(text: str, values: list[str]) -> list[str]:
    return [value for value in values if value not in text]


def run(command: list[str], cwd: Path, env: dict[str, str] | None = None) -> tuple[bool, str]:
    result = subprocess.run(command, cwd=cwd, env=env, capture_output=True, text=True, check=False)
    return result.returncode == 0, (result.stdout + result.stderr)[-6000:]


def cargo_env(manifest: Path) -> dict[str, str]:
    digest = str(abs(hash(str(manifest.resolve()))))
    home = Path(tempfile.gettempdir()) / f"exact-list-specular-{digest}"
    result = subprocess.run(
        [
            "python3",
            str(SHACKLES / "scripts/local_cargo_source.py"),
            "--root",
            str(ROOT),
            "--config",
            str(home / "config.toml"),
            "--manifest",
            str(manifest),
        ],
        capture_output=True,
        text=True,
        check=False,
    )
    if result.returncode != 0:
        raise RuntimeError(result.stdout + result.stderr)
    environment = os.environ.copy()
    environment["CARGO_HOME"] = str(home)
    return environment


def changed_paths() -> set[str]:
    tracked = subprocess.run(
        ["git", "diff", "--name-only", "fb52d87", "--"],
        cwd=ROOT,
        capture_output=True,
        text=True,
        check=True,
    )
    untracked = subprocess.run(
        ["git", "ls-files", "--others", "--exclude-standard"],
        cwd=ROOT,
        capture_output=True,
        text=True,
        check=True,
    )
    return set(tracked.stdout.splitlines()) | set(untracked.stdout.splitlines())


def cspell_deny_change(path: str) -> bool:
    result = subprocess.run(
        ["git", "diff", "--unified=0", "fb52d87", "--", path],
        cwd=ROOT, capture_output=True, text=True, check=True,
    )
    lines = [
        line[1:].strip()
        for line in result.stdout.splitlines()
        if line.startswith(("+", "-")) and not line.startswith(("+++", "---")) and line[1:].strip()
    ]
    return bool(lines) and all("shakts-cspell-" in line for line in lines)


def aqc_lock_change(path: str) -> bool:
    try:
        old_text = subprocess.run(
            ["git", "show", f"fb52d87:{path}"],
            cwd=ROOT, capture_output=True, text=True, check=True,
        ).stdout
        old = tomllib.loads(old_text)
        new = tomllib.loads((ROOT / path).read_text())
    except (subprocess.CalledProcessError, tomllib.TOMLDecodeError, OSError):
        return False
    old_top_level = {key: value for key, value in old.items() if key != "package"}
    new_top_level = {key: value for key, value in new.items() if key != "package"}
    if old_top_level != new_top_level:
        return False
    old_records = old.get("package", [])
    new_records = new.get("package", [])
    if len(old_records) != len(new_records):
        return False

    old_by_name = {record["name"]: record for record in old_records}
    new_by_name = {record["name"]: record for record in new_records}
    if old_by_name.keys() != new_by_name.keys() or len(old_by_name) != len(old_records):
        return False

    changed = False
    for name, old_record in old_by_name.items():
        new_record = new_by_name[name]
        if old_record == new_record:
            continue
        if not name.startswith("aqc-"):
            return False
        old_without_version = {key: value for key, value in old_record.items() if key != "version"}
        new_without_version = {key: value for key, value in new_record.items() if key != "version"}
        if old_without_version != new_without_version:
            return False
        try:
            old_version = tuple(int(part) for part in old_record["version"].split("."))
            new_version = tuple(int(part) for part in new_record["version"].split("."))
        except (KeyError, ValueError):
            return False
        if len(old_version) != 3 or len(new_version) != 3:
            return False
        if old_version[:2] != new_version[:2] or new_version[2] <= old_version[2]:
            return False
        changed = True
    return changed


ok = True
detail = ""

if CHECK == "difference-contract":
    text = source(CORE)
    tests = source(CORE / "tests")
    missing = require(
        text + tests,
        [
            "pub struct ExactListDifference",
            "pub fn exact_list_difference",
            "pub fn apply_list_requirements",
            "saturating_sub",
            "order_mismatch",
            "pub fn current_count",
            "pub fn expected_count",
            "exact_list_difference_covers_membership_duplicates_empty_values_and_order",
            "exact_list_difference_orders_distinct_members_lexically",
            "apply_list_requirements_uses_exact_then_contains_then_excludes",
        ],
    )
    forbidden = [value for value in ["Finding::", "toml_edit", "yaml_edit"] if value in (CORE / "src/merge/lists.rs").read_text()]
    ok = not missing and not forbidden
    detail = f"missing={missing}; forbidden={forbidden}"
elif CHECK in {
    "json-reconciliation-contract",
    "toml-reconciliation-contract",
    "yaml-reconciliation-contract",
    "preserved-requirement-contract",
}:
    roots = {
        "json-reconciliation-contract": [JSON_ENGINE],
        "toml-reconciliation-contract": [TOML_CORE, ROOT / "packages/file-types/toml"],
        "yaml-reconciliation-contract": [YAML_ENGINE],
        "preserved-requirement-contract": [
            CORE,
            JSON_ENGINE,
            ROOT / "packages/file-types/toml",
            YAML_ENGINE,
        ],
    }[CHECK]
    text = "\n".join(source(root) for root in roots)
    required = {
        "json-reconciliation-contract": [
            "exact_list_findings_are_member_specific_order_aware_and_constructive",
            "exact_and_glob_findings_keep_member_selectors_including_empty_values",
            "compatible_exact_member_assertions_share_json_selector_identity",
        ],
        "toml-reconciliation-contract": [
            "exact_list_findings_are_member_specific_order_aware_and_presence_aware",
            "compatible_exact_member_assertions_share_toml_member_identity",
            "cargo_list_fields_report_malformed_shape_without_treating_it_as_absent",
            "let missing = reconcile_optional_list_field",
        ],
        "yaml-reconciliation-contract": [
            "exact_list_differences_are_member_specific_order_aware_and_attributed",
            "compatible_exact_member_assertions_share_selector_identity",
        ],
        "preserved-requirement-contract": [
            "list_contains_and_excludes_reconcile_values",
            "forbidden_ignore_path_glob_removes_matching_values",
            "exact_list_conflict_uses_stable_contributor_text",
        ],
    }[CHECK]
    missing = require(text, required)
    ok = not missing
    detail = f"missing contract tests={missing}"
elif CHECK == "single-difference-implementation":
    files = [
        JSON_ENGINE / "src/runtime/reconcile/lists.rs",
        TOML_CORE / "src/lists.rs",
        YAML_ENGINE / "src/runtime/reconcile/apply.rs",
    ]
    missing = []
    for path in files:
        text = path.read_text()
        for call in ("exact_list_difference", "apply_list_requirements"):
            if call not in text:
                missing.append(f"{path.relative_to(ROOT)}:{call}")
    local_helpers = []
    for path in files:
        text = path.read_text()
        for name in ("fn exact_list_difference", "fn desired_list", "fn member_count"):
            if name in text:
                local_helpers.append(f"{path.relative_to(ROOT)}:{name}")
    ok = not missing and not local_helpers
    detail = f"missing={missing}; local_duplicates={local_helpers}"
elif CHECK == "optional-toml-callers":
    paths = [
        "packages/file-types/toml/aqc-cargo-toml-engine/src/reconcile/package_fields.rs",
        "packages/file-types/toml/aqc-cargo-toml-engine/src/reconcile/target_tables.rs",
        "packages/file-types/toml/aqc-cargo-toml-engine/src/reconcile/workspace_fields.rs",
        "packages/file-types/toml/aqc-deny-toml-engine/src/reconcile/lists.rs",
        "packages/file-types/toml/aqc-rust-toolchain-toml-engine/src/reconcile/settings.rs",
        "packages/file-types/toml/aqc-rustfmt-toml-engine/src/reconcile/settings/list.rs",
    ]
    missing = [path for path in paths if "reconcile_optional" not in (ROOT / path).read_text()]
    for path in paths[:3]:
        if "report_list_item_shape" not in (ROOT / path).read_text():
            missing.append(f"{path}:shape")
    ok = not missing
    detail = f"callers without optional reconciliation={missing}"
elif CHECK == "fixture3-contract":
    ok, detail = run(["fixture3", "check", "--suite", "exact-list-differences"], ROOT)
elif CHECK == "workspace-gates":
    failures = []
    for workspace in WORKSPACES:
        environment = cargo_env(workspace / "Cargo.toml")
        for command in (
            ["g3rs", "validate", "workspace", "--path", ".", "--rules-only"],
            ["cargo", "+1.88.0", "fmt", "--all", "--", "--check"],
            ["cargo", "+1.88.0", "test", "--all-targets", "--all-features", "--locked"],
            ["cargo", "+1.88.0", "clippy", "--all-targets", "--all-features", "--locked", "--", "-D", "warnings"],
            ["cargo", "+1.88.0", "deny", "check"],
            ["cargo", "+1.88.0", "package", "--allow-dirty", "--locked"],
        ):
            passed, output = run(command, workspace, environment)
            if not passed:
                failures.append(f"{workspace.name}: {' '.join(command)}\n{output}")
    ok = not failures
    detail = "\n".join(failures)
elif CHECK == "dependency-generation":
    expected = {
        "packages/aqc-file-engine-core/Cargo.toml": "0.7.2",
        "packages/file-types/json/aqc-json-file-engine/Cargo.toml": "0.1.1",
        "packages/file-types/toml/aqc-toml-engine-core/Cargo.toml": "0.8.1",
        "packages/file-types/toml/aqc-cargo-toml-engine/Cargo.toml": "0.7.2",
        "packages/file-types/toml/aqc-deny-toml-engine/Cargo.toml": "0.7.2",
        "packages/file-types/toml/aqc-rust-toolchain-toml-engine/Cargo.toml": "0.7.2",
        "packages/file-types/toml/aqc-rustfmt-toml-engine/Cargo.toml": "0.7.2",
        "packages/file-types/yaml/aqc-pnpm-workspace-yaml-engine/Cargo.toml": "0.7.2",
    }
    errors = []
    for relative, version in expected.items():
        document = tomllib.loads((ROOT / relative).read_text())
        if document["package"]["version"] != version:
            errors.append(f"{relative}: expected {version}")
        if "path =" in (ROOT / relative).read_text():
            errors.append(f"{relative}: path dependency")
    ok = not errors
    detail = "; ".join(errors)
elif CHECK == "unchanged-dependency-only-packages":
    fragments = [f"/{package}/Cargo.toml" for package in ENTRY["packages"]]
    changed = changed_paths()
    violations = [path for path in changed if any(fragment in f"/{path}" for fragment in fragments)]
    ok = not violations
    detail = f"changed dependency-only manifests={violations}"
elif CHECK == "changed-path-scope":
    tree = set(SPEC["requirements"]["tree"]["required"])
    package_roots = [
        "packages/aqc-file-engine-core",
        "packages/file-types/json/aqc-json-file-engine",
        "packages/file-types/toml/aqc-toml-engine-core",
        "packages/file-types/toml/aqc-cargo-toml-engine",
        "packages/file-types/toml/aqc-deny-toml-engine",
        "packages/file-types/toml/aqc-rust-toolchain-toml-engine",
        "packages/file-types/toml/aqc-rustfmt-toml-engine",
        "packages/file-types/yaml/aqc-pnpm-workspace-yaml-engine",
    ]
    package_metadata = {
        f"{root}/{name}"
        for root in package_roots
        for name in ("Cargo.toml", "Cargo.lock")
    }
    allowed_exact = tree | package_metadata | {
        ".plans/2026-07-15-211236-exact-list-differences.md",
    }
    violations = sorted(
        path
        for path in changed_paths()
        if (path in DEPENDENCY_ONLY_LOCKS and not aqc_lock_change(path))
        or (
            path not in DEPENDENCY_ONLY_LOCKS
            and path not in allowed_exact
            and not path.startswith(".worklogs/")
            and not (path.startswith("packages/") and path.endswith("/deny.toml") and cspell_deny_change(path))
            and not (
                path.startswith("packages/")
                and path.endswith("/Cargo.lock")
                and cspell_deny_change(path.removesuffix("Cargo.lock") + "deny.toml")
                and aqc_lock_change(path)
            )
        )
    )
    ok = not violations
    detail = f"out-of-scope paths={violations}"
else:
    raise SystemExit(f"unknown custom check: {CHECK}")

print(json.dumps({"check": CHECK, "status": "pass" if ok else "fail", "message": detail}))
