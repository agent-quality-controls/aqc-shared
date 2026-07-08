#!/usr/bin/env python3
from __future__ import annotations

import json
import subprocess
import sys
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]


def main() -> int:
    spec = json.loads(Path(sys.argv[1]).read_text())
    entry = spec["requirements"]["custom"][int(sys.argv[3])]
    check = entry["check"]
    if check == "cargo-workspaces":
        print(json.dumps(cargo_workspaces(), sort_keys=True))
    elif check == "text-core-contract":
        print(json.dumps(text_core_contract(), sort_keys=True))
    else:
        print(json.dumps(fail(f"unknown check {check}", check=check), sort_keys=True))
    return 0


def ok(**extra: object) -> dict[str, object]:
    return {"status": "pass", **extra}


def fail(message: str, **extra: object) -> dict[str, object]:
    return {"status": "fail", "message": message, **extra}


def run(argv: list[str], cwd: Path) -> dict[str, object]:
    completed = subprocess.run(argv, cwd=cwd, text=True, capture_output=True, check=False)
    if completed.returncode == 0:
        return ok(check=" ".join(argv), workspace=str(cwd.relative_to(ROOT)))
    return fail(
        f"{' '.join(argv)} failed with exit {completed.returncode}",
        check=" ".join(argv),
        workspace=str(cwd.relative_to(ROOT)),
        stdout=completed.stdout[-3000:],
        stderr=completed.stderr[-3000:],
        exit_code=completed.returncode,
    )


def cargo_workspaces() -> dict[str, object]:
    workspaces = [
        ROOT / "packages/aqc-file-engine-core",
        ROOT / "packages/file-types/text/aqc-text-engine-core",
        ROOT / "packages/file-types/git/aqc-git-hooks-engine",
    ]
    missing = [str(path.relative_to(ROOT)) for path in workspaces if not (path / "Cargo.toml").exists()]
    if missing:
        return fail("missing expected Cargo workspaces", missing=missing)
    failures = []
    for workspace in workspaces:
        result = run(["cargo", "test", "--locked"], workspace)
        if result["status"] != "pass":
            failures.append(result)
    if failures:
        return fail("one or more AQC Cargo workspaces failed", failures=failures)
    return ok(check="cargo-workspaces")


def text_core_contract() -> dict[str, object]:
    tests = ROOT / "packages/file-types/text/aqc-text-engine-core/tests"
    if not tests.exists():
        return fail("missing text core tests directory")
    text = "\n".join(path.read_text(errors="replace") for path in sorted(tests.glob("*.rs")))
    required_tests = [
        "uses_core_item_merge_for_files",
        "uses_core_item_merge_for_snippets",
        "exact_contents_mismatch_reports",
        "missing_snippet_reports",
        "missing_executable_reports",
        "init_writes_expected_bytes",
    ]
    missing = [name for name in required_tests if name not in text]
    if missing:
        return fail("missing required text core contract tests", missing=missing)
    cargo = (ROOT / "packages/file-types/git/aqc-git-hooks-engine/Cargo.toml").read_text()
    if "aqc-text-file-engine" in cargo:
        return fail("git hooks engine still depends on text file engine")
    return ok(check="text-core-contract")


if __name__ == "__main__":
    raise SystemExit(main())
