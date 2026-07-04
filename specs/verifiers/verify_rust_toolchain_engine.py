#!/usr/bin/env python3
from __future__ import annotations

import json
import subprocess
import sys
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[2]
PKG = "packages/file-types/toml/aqc-rust-toolchain-toml-engine"


def evidence(status: str, message: str | None = None, **extra: Any) -> None:
    out: dict[str, Any] = {"status": status}
    if message:
        out["message"] = message
    out.update(extra)
    print(json.dumps(out, sort_keys=True))


def read_entry() -> dict[str, Any]:
    spec = json.loads(Path(sys.argv[1]).read_text())
    return spec["requirements"]["custom"][int(sys.argv[3])]


def git_changed_files() -> list[str]:
    proc = subprocess.run(
        ["git", "status", "--short"],
        cwd=ROOT,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        check=False,
    )
    if proc.returncode != 0:
        raise RuntimeError(proc.stderr.strip() or "git status failed")
    files: list[str] = []
    for line in proc.stdout.splitlines():
        if not line.strip():
            continue
        path = line[3:]
        if " -> " in path:
            path = path.split(" -> ", 1)[1]
        files.append(path)
    return sorted(files)


def changed_file_scope() -> None:
    allowed_prefixes = (
        f"{PKG}/",
        "specs/",
        ".worklogs/",
    )
    bad = [
        path
        for path in git_changed_files()
        if not path.startswith(allowed_prefixes)
    ]
    if bad:
        evidence("fail", "changed files outside allowed AQC engine scope", check="changed-file-scope", files=bad)
    else:
        evidence("pass", check="changed-file-scope", changed=git_changed_files())


def symbol_scope() -> None:
    allowed_prefixes = (
        f"{PKG}/",
        "specs/",
        ".worklogs/",
    )
    needles = (
        "aqc-rust-toolchain-toml-engine",
        "aqc_rust_toolchain_toml_engine",
        "RustToolchainTomlEngine",
        "RustToolchainTomlRequirements",
        "ResolvedRustToolchainTomlRequirements",
        "RustToolchainScalarSetting",
        "RustToolchainListSetting",
    )
    bad: list[str] = []
    for path in ROOT.rglob("*"):
        if not path.is_file():
            continue
        rel = path.relative_to(ROOT).as_posix()
        if rel.startswith(".git/") or rel.startswith("target/") or "/target/" in rel:
            continue
        if rel.startswith(allowed_prefixes):
            continue
        try:
            body = path.read_text(errors="ignore")
        except OSError:
            continue
        matches = [needle for needle in needles if needle in body]
        if matches:
            bad.append(f"{rel}: {', '.join(matches)}")
    if bad:
        evidence("fail", "rust-toolchain engine symbols outside approved scope", check="symbol-scope", files=bad)
    else:
        evidence("pass", check="symbol-scope")


def read(path: str, failures: list[str]) -> str:
    full = ROOT / path
    if not full.exists():
        failures.append(f"missing {path}")
        return ""
    return full.read_text()


def engine_contract() -> None:
    failures: list[str] = []
    model = read(f"{PKG}/src/requirement/model.rs", failures)
    settings = read(f"{PKG}/src/requirement/settings.rs", failures)
    engine = read(f"{PKG}/src/engine.rs", failures)
    reconcile_tests = read(f"{PKG}/tests/reconcile.rs", failures)

    for item in (
        "pub struct RustToolchainTomlRequirements",
        "pub scalar_settings: RustToolchainScalarSettings",
        "pub list_settings: BTreeMap<RustToolchainListSetting, ListRequirements>",
        "pub closed_settings: Option<String>",
        "pub struct ResolvedRustToolchainTomlRequirements",
        "pub scalar_settings: ResolvedRustToolchainScalarSettings",
        "pub list_settings: BTreeMap<RustToolchainListSetting, ResolvedListRequirements>",
        "pub closed_settings: ResolvedRustToolchainClosedSettings",
        "impl EngineRequirement for RustToolchainTomlRequirements",
    ):
        if item not in model:
            failures.append(f"model missing {item}")

    for item in (
        "ToolchainRequirements",
        "ResolvedToolchainRequirements",
        "scalar_fields",
        "list_fields",
        "closed_toolchain_fields",
        "CargoRustVersionRequirement",
    ):
        if item in model:
            failures.append(f"model contains forbidden {item}")

    for item in (
        "pub enum RustToolchainScalarSetting",
        "Channel",
        "Path",
        "Profile",
        "pub enum RustToolchainListSetting",
        "Components",
        "Targets",
        "file_key",
    ):
        if item not in settings:
            failures.append(f"settings missing {item}")

    for item in (
        "workspace_root.join(\"rust-toolchain.toml\")",
        "merged_reconcile",
        "RustToolchainTomlRequirements::merge",
    ):
        if item not in engine:
            failures.append(f"engine missing {item}")

    for item in (
        "missing_file",
        "writes_deterministic_file",
        "missing_toolchain_table",
        "wrong_channel",
        "missing_component",
        "channel_and_path_conflict",
        "path_blocks_components",
        "relative_path",
        "invalid_profile",
        "empty_toolchain_table",
        "closed_settings",
        "unknown_setting_allowed_when_open",
        "list_order_is_ignored",
        "path_blocks_targets",
        "path_blocks_profile",
        "path_requirement_skips_channel_based_writes",
        "path_absent_requirement_allows_channel",
        "relative_path_requirement_is_not_written",
    ):
        if item not in reconcile_tests:
            failures.append(f"reconcile tests missing {item}")

    if failures:
        evidence("fail", "engine contract failed", check="engine-contract", failures=failures)
    else:
        evidence("pass", check="engine-contract")


def cargo_tests() -> None:
    manifest = ROOT / PKG / "Cargo.toml"
    if not manifest.exists():
        evidence("fail", "missing engine Cargo.toml", check="cargo-tests")
        return
    proc = subprocess.run(
        ["cargo", "test", "--manifest-path", str(manifest), "--all-targets"],
        cwd=ROOT,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        check=False,
    )
    if proc.returncode != 0:
        evidence(
            "fail",
            "cargo test failed",
            check="cargo-tests",
            exit=proc.returncode,
            stdout=proc.stdout[-1000:],
            stderr=proc.stderr[-1000:],
        )
    else:
        evidence("pass", check="cargo-tests")


def main() -> int:
    entry = read_entry()
    check = entry.get("check")
    try:
        if check == "changed-file-scope":
            changed_file_scope()
        elif check == "symbol-scope":
            symbol_scope()
        elif check == "engine-contract":
            engine_contract()
        elif check == "cargo-tests":
            cargo_tests()
        else:
            evidence("fail", f"unknown check {check}", check=check)
    except Exception as exc:
        evidence("fail", f"{type(exc).__name__}: {exc}", check=check)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
