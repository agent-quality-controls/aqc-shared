#!/usr/bin/env python3
import json
import subprocess
import sys
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
PLAN = Path("/Users/tartakovsky/Projects/agent-quality-controls/shackles/.plans/2026-07-11-213527-create-only-init-and-exact-items.md")

def emit(check: str, passed: bool, message: str) -> None:
    print(json.dumps({"check": check, "status": "pass" if passed else "fail", "message": message}))


def run(argv: list[str], cwd: Path, timeout: int = 50) -> tuple[bool, str]:
    result = subprocess.run(argv, cwd=cwd, text=True, capture_output=True, timeout=timeout)
    output = (result.stdout + result.stderr).strip()
    return result.returncode == 0, output[-3000:]


def exact_item_semantics() -> tuple[bool, str]:
    return run(
        ["cargo", "test", "--locked", "--test", "exact_items"],
        ROOT / "packages/aqc-file-engine-core",
    )


def engine_semantics() -> tuple[bool, str]:
    targets = [
        ("packages/file-types/text/aqc-text-engine-core", ["cargo", "test", "--locked", "--test", "reconcile_tests"]),
        ("packages/file-types/toml/aqc-toml-engine-core", ["cargo", "test", "--locked"]),
        ("packages/file-types/toml/aqc-cargo-toml-engine", ["cargo", "test", "--locked", "--test", "package_lint_tables"]),
        ("packages/file-types/toml/aqc-clippy-toml-engine", ["cargo", "test", "--locked", "--test", "merge"]),
        ("packages/file-types/toml/aqc-deny-toml-engine", ["cargo", "test", "--locked", "--test", "reconcile"]),
        ("packages/file-types/toml/aqc-rustfmt-toml-engine", ["cargo", "test", "--locked", "--test", "reconcile_ignore_exact"]),
        ("packages/file-types/toml/aqc-rust-toolchain-toml-engine", ["cargo", "test", "--locked", "--test", "behavior"]),
    ]
    for relative, command in targets:
        passed, output = run(command, ROOT / relative)
        if not passed:
            return False, f"{relative}: {output}"
    return True, "all exact reconciliation suites pass"


def workspace_gates(relative: str) -> tuple[bool, str]:
    core_model = (ROOT / "packages/aqc-file-engine-core/src/merge/model.rs").read_text()
    if "pub exact: Option<ExactItems<Item>>" not in core_model:
        return False, "exact-item implementation is absent"
    cwd = ROOT / relative
    commands = [
        ["cargo", "test", "--locked", "--all-features"],
        ["cargo", "clippy", "--locked", "--all-targets", "--all-features", "--", "-D", "warnings"],
        ["cargo", "deny", "check"],
        ["cargo", "package", "--locked", "--allow-dirty"],
        ["cargo", "+1.85", "check", "--locked", "--all-features"],
    ]
    for command in commands:
        passed, output = run(command, cwd)
        if not passed:
            return False, f"{relative}: {' '.join(command)}: {output}"
    return True, f"{relative}: all workspace gates pass"


def release_dependency_versions() -> tuple[bool, str]:
    expected = {
        "packages/file-types/toml/aqc-toml-engine-core/Cargo.toml": "0.5.0",
        "packages/file-types/text/aqc-text-engine-core/Cargo.toml": "0.5.0",
        "packages/file-types/toml/aqc-cargo-toml-engine/Cargo.toml": "0.5.0",
        "packages/file-types/toml/aqc-clippy-toml-engine/Cargo.toml": "0.5.0",
        "packages/file-types/toml/aqc-deny-toml-engine/Cargo.toml": "0.5.0",
        "packages/file-types/toml/aqc-rustfmt-toml-engine/Cargo.toml": "0.5.0",
        "packages/file-types/toml/aqc-rust-toolchain-toml-engine/Cargo.toml": "0.5.0",
    }
    failures = []
    for relative, core_version in expected.items():
        text = (ROOT / relative).read_text()
        if "path =" in text:
            failures.append(f"{relative}: path dependency remains")
        if f'aqc-file-engine-core = "{core_version}"' not in text:
            failures.append(f"{relative}: aqc-file-engine-core {core_version} not pinned")
    return not failures, "registry dependency versions match release order" if not failures else "; ".join(failures)


def changed_paths() -> set[str]:
    commands = [
        ["git", "diff", "--name-only"],
        ["git", "diff", "--cached", "--name-only"],
        ["git", "ls-files", "--others", "--exclude-standard"],
    ]
    paths = set()
    for command in commands:
        result = subprocess.run(command, cwd=ROOT, text=True, capture_output=True)
        if result.returncode == 0:
            paths.update(line for line in result.stdout.splitlines() if line)
    return paths


def change_scope() -> tuple[bool, str]:
    allowed = (
        "packages/aqc-file-engine-core/",
        "packages/file-types/toml/aqc-toml-engine-core/",
        "packages/file-types/text/aqc-text-engine-core/",
        "packages/file-types/toml/aqc-cargo-toml-engine/",
        "packages/file-types/toml/aqc-clippy-toml-engine/",
        "packages/file-types/toml/aqc-deny-toml-engine/",
        "packages/file-types/toml/aqc-rustfmt-toml-engine/",
        "packages/file-types/toml/aqc-rust-toolchain-toml-engine/",
        "specs/",
        ".plans/",
        ".worklogs/",
    )
    outside = sorted(path for path in changed_paths() if not path.startswith(allowed))
    return not outside, "changed paths are within AQC scope" if not outside else "out-of-scope paths: " + ", ".join(outside)


def coverage_map() -> tuple[bool, str]:
    coverage = (ROOT / "specs/create-only-init-and-exact-items.spec.coverage.md").read_text()
    plan = PLAN.read_text()
    aqc_headings = [
        "## Goal",
        "## Decisions",
        "### Init owns creation only",
        "### Remove init commands",
        "### Exact item vocabulary",
        "### Cargo package lint table presence",
        "## AQC Changes",
        "### `aqc-file-engine-core`",
        "### `aqc-toml-engine-core`",
        "### `aqc-text-engine-core`",
        "### Cargo TOML engine",
        "### Other AQC engines",
        "## Shackles Changes",
        "## Fixture Requirements",
        "## Specular Specifications",
        "### AQC spec",
        "### Shackles spec",
        "## Migration And Release Order",
        "## Resume Repository Adoption",
        "## Completion Gates",
    ]
    missing_plan = [heading for heading in aqc_headings if heading not in plan]
    missing_coverage = [heading for heading in aqc_headings if heading not in coverage]
    passed = not missing_plan and not missing_coverage
    message = "every AQC-applicable heading is mapped" if passed else f"missing plan={missing_plan}; missing coverage={missing_coverage}"
    return passed, message


def main() -> None:
    spec = json.loads(Path(sys.argv[1]).read_text())
    index = int(sys.argv[3])
    check = spec["requirements"]["custom"][index]["check"]
    handlers = {
        "exact-item-semantics": exact_item_semantics,
        "engine-reconciliation-semantics": engine_semantics,
        "release-dependency-versions": release_dependency_versions,
        "change-scope": change_scope,
        "coverage-map": coverage_map,
    }
    passed, message = workspace_gates(spec["requirements"]["custom"][index]["workspace"]) if check == "workspace-gates" else handlers[check]()
    emit(check, passed, message)


if __name__ == "__main__":
    main()
