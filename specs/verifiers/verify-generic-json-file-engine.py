#!/usr/bin/env python3
from __future__ import annotations

import json
import os
import subprocess
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SHACKLES = ROOT.parent / "shackles"
spec = json.loads(Path(sys.argv[1]).read_text())
entry = spec["requirements"]["custom"][int(sys.argv[3])]
check = entry["check"]
engine = ROOT / "packages/file-types/json/aqc-json-file-engine"
core = ROOT / "packages/file-types/json/aqc-json-engine-core"
file_core = ROOT / "packages/aqc-file-engine-core"
package_json = ROOT / "packages/file-types/json/aqc-package-json-engine"
tsconfig = ROOT / "packages/file-types/jsonc/aqc-tsconfig-json-engine"


def source_text(root: Path) -> str:
    return "\n".join(path.read_text() for path in sorted(root.rglob("*.rs"))) if root.exists() else ""


def cargo_environment(manifest: Path) -> dict[str, str]:
    home = Path(tempfile.gettempdir()) / "aqc-json-file-engine-specular-cargo-home"
    result = subprocess.run(
        [
            "python3", str(SHACKLES / "scripts/local_cargo_source.py"),
            "--root", str(SHACKLES), "--config", str(home / "config.toml"),
            "--manifest", str(manifest),
        ],
        capture_output=True, text=True, check=False,
    )
    if result.returncode != 0:
        raise RuntimeError(result.stdout + result.stderr)
    environment = os.environ.copy()
    environment["CARGO_HOME"] = str(home)
    return environment


def run(command: list[str], cwd: Path, env: dict[str, str] | None = None) -> tuple[bool, str]:
    result = subprocess.run(command, cwd=cwd, env=env, text=True, capture_output=True, check=False)
    return result.returncode == 0, (result.stdout + result.stderr)[-4000:]


if check == "public-api":
    joined = source_text(engine / "src")
    facade = (engine / "src/lib.rs").read_text()
    required = [
        "pub const fn root() -> Self",
        "pub fn new(first: impl Into<String>) -> Self",
        "pub fn child(mut self, component: impl Into<String>) -> Self",
        "pub fn components(&self) -> impl Iterator<Item = &str>",
        "pub fn pointer(&self) -> String",
        "pub fn selector(&self) -> String",
        "pub struct JsonStringGlob",
        "pub struct JsonFileRequirements",
        "pub scalar_values: BTreeMap<JsonPath, ScalarAssertion<ConfigScalar>>",
        "pub string_lists: BTreeMap<JsonPath, ListRequirements>",
        "pub forbidden_string_list_globs: BTreeMap<JsonPath, ForbiddenGlobRequirements<JsonStringGlob>>",
        "pub object_keys: BTreeMap<JsonPath, ItemRequirements<KeyedItem<()>>>",
        "pub struct ResolvedJsonFileRequirements",
        "pub struct JsonFileEngine",
        "impl FindingKey for JsonPath",
        "impl EngineRequirement for JsonFileRequirements",
        "impl FileEngine<ResolvedJsonFileRequirements> for JsonFileEngine",
        "impl Engine for JsonFileEngine",
        "ScalarValue",
        "push_rendered_conflict",
    ]
    forbidden = [
        "pub scalar_values: ResolvedMap",
        "pub string_lists: BTreeMap<JsonPath, ResolvedListRequirements>",
    ]
    forbidden_exports = [
        "ResolvedForbiddenGlobRequirements", "ResolvedItemRequirements",
        "ResolvedListRequirements", "ResolvedRequirement",
    ]
    missing = [item for item in required if item not in joined]
    present = [item for item in forbidden if item in joined]
    exported = [item for item in forbidden_exports if item in facade]
    ok = not missing and not present and not exported
    detail = f"missing={missing}; exposed_resolved_fields={present}; forbidden_exports={exported}"
elif check == "behavior-contract":
    joined = (
        source_text(engine / "tests")
        + source_text(core / "tests")
        + source_text(file_core / "tests")
    ).lower()
    required = [
        "strict_duplicate_non_object_non_string_and_invalid_utf8_inputs_fail_closed",
        "exact_root_keys_report_and_remove_only_extra_keys",
        "exact_root_keys_accept_a_matching_root_without_a_shape_finding",
        "list_creation_distinguishes_exact_empty_from_non_constructive_requirements",
        "malformed_list_and_blocked_parent_are_reported_without_rewriting_bytes",
        "invalid_glob_fails_closed_with_policy_attribution",
        "merge_rejects_leaf_descendants_and_required_items_forbidden_by_glob",
        "missing_document_generation_is_idempotent_and_preserves_json_pointer_identity",
        "scalar_and_string_list_at_same_path_conflict_with_provenance",
        "forbidden_glob_reports_each_selector_and_removes_matches",
        "object_closure_cannot_exclude_a_managed_descendant",
        "invalid_glob_prevents_every_other_edit",
        "exact_empty_nested_object_is_initialized",
        "overlapping_globs_combine_attribution_and_duplicate_values_report_once",
        "root_leaf_requirements_are_rejected",
        "object_creation_uses_shared_parent_write_rules",
        "descendant_objects_are_created_before_ancestor_key_checks",
        "empty_collection_requirements_are_no_ops",
        "collection_findings_distinguish_whole_collection_from_empty_members",
        "collection_merge_conflicts_use_json_pointer_member_keys",
        "blocked_object_parent_reports_one_shape_finding_without_rewriting_bytes",
        "object_membership_conflicts_only_with_descendants_that_require_presence",
        "same_surface_presence_conflicts_are_reported_only_by_core",
        "same_surface_presence_conflict_does_not_hide_kind_conflict",
        "nonconstructive_requirement_does_not_duplicate_presence_conflict",
        "kind_conflict_attributes_only_unexplained_kind_contributors",
        "rendered_conflict_contributors_are_deterministic",
        "same_object_membership_conflicts_are_reported_only_by_core",
        "required_glob_conflict_is_unique_complete_and_member_keyed",
    ]
    missing = [item for item in required if item not in joined]
    ok = not missing
    detail = f"missing behavior contracts={missing}"
elif check in {"file-core-gates", "core-gates", "engine-gates", "package-json-gates", "tsconfig-gates"}:
    failures = []
    workspace = {
        "file-core-gates": file_core,
        "core-gates": core,
        "engine-gates": engine,
        "package-json-gates": package_json,
        "tsconfig-gates": tsconfig,
    }[check]
    env = cargo_environment(workspace / "Cargo.toml")
    commands = [
        ["cargo", "+1.88.0", "fmt", "--all", "--", "--check"],
        ["cargo", "+1.88.0", "test", "--all-features", "--locked"],
        ["cargo", "+1.88.0", "clippy", "--all-targets", "--all-features", "--locked"],
        ["cargo", "+1.88.0", "deny", "check"],
        ["cargo", "+1.88.0", "package", "--allow-dirty", "--locked"],
    ]
    for command in commands:
        passed, output = run(command, workspace, env)
        if not passed:
            failures.append(f"{workspace.name}: {' '.join(command)}: {output}")
    ok = not failures
    detail = "\n".join(failures)
elif check == "affected-caller-gate":
    workspace = ROOT / entry["workspace"]
    env = cargo_environment(workspace / "Cargo.toml")
    ok, detail = run(
        ["cargo", "+1.88.0", "check", "--all-targets", "--all-features", "--locked"],
        workspace,
        env,
    )
elif check == "fixture-gate":
    passed, output = run(["fixture3", "check", "--suite", "generic-json-file-engine"], ROOT)
    ok = passed
    detail = output
elif check == "changed-path-scope":
    result = subprocess.run(
        ["git", "diff", "--name-only", "d86a987", "--"],
        cwd=ROOT, text=True, capture_output=True, check=False,
    )
    untracked = subprocess.run(
        ["git", "ls-files", "--others", "--exclude-standard"],
        cwd=ROOT, text=True, capture_output=True, check=False,
    )
    allowed_exact = {
        ".github/workflows/release.yml",
        ".plans/2026-07-15-142457-generic-json-file-engine.md",
        ".worklogs/2026-07-15-161958-generic-json-file-engine.md",
        "fixture3.yaml",
        "fixtures/approved/generic-json-file-engine/approved.meta.json",
        "fixtures/approved/generic-json-file-engine/approved.normalized.json",
        "fixtures/generic-json-file-engine/contracts.json",
        "fixtures/probes/generic-json-file-engine/Cargo.lock",
        "fixtures/probes/generic-json-file-engine/Cargo.toml",
        "fixtures/probes/generic-json-file-engine/guardrail3-rs.toml",
        "fixtures/probes/generic-json-file-engine/src/main.rs",
        "fixtures/scripts/fixture3-generic-json-file-engine.py",
        "release-plz.toml",
        "packages/file-types/json/aqc-json-file-engine/Cargo.lock",
        "packages/file-types/json/aqc-json-file-engine/Cargo.toml",
        "packages/file-types/json/aqc-json-file-engine/LICENSE",
        "packages/file-types/json/aqc-json-file-engine/README.md",
        "packages/file-types/json/aqc-json-file-engine/deny.toml",
        "packages/file-types/json/aqc-json-file-engine/guardrail3-rs.toml",
        "packages/file-types/json/aqc-json-file-engine/src/lib.rs",
        "packages/file-types/json/aqc-json-file-engine/src/runtime/engine.rs",
        "packages/file-types/json/aqc-json-file-engine/src/runtime/merge/collect.rs",
        "packages/file-types/json/aqc-json-file-engine/src/runtime/merge/conflicts.rs",
        "packages/file-types/json/aqc-json-file-engine/src/runtime/merge/mod.rs",
        "packages/file-types/json/aqc-json-file-engine/src/runtime/merge/required_globs.rs",
        "packages/file-types/json/aqc-json-file-engine/src/runtime/merge/resolve.rs",
        "packages/file-types/json/aqc-json-file-engine/src/runtime/mod.rs",
        "packages/file-types/json/aqc-json-file-engine/src/runtime/reconcile.rs",
        "packages/file-types/json/aqc-json-file-engine/src/types/mod.rs",
        "packages/file-types/json/aqc-json-file-engine/src/types/model.rs",
        "packages/file-types/json/aqc-json-file-engine/tests/contract.rs",
        "packages/file-types/json/aqc-json-file-engine/tests/engine_requirement.rs",
        "specs/generic-json-file-engine.spec.coverage.md",
        "specs/generic-json-file-engine.spec.json",
        "specs/verifiers/verify-generic-json-file-engine.py",
        "packages/file-types/json/aqc-json-engine-core/src/runtime/scalar.rs",
        "packages/file-types/json/aqc-json-engine-core/src/types/object.rs",
        "packages/file-types/json/aqc-json-engine-core/tests/core_contract.rs",
        "packages/file-types/json/aqc-package-json-engine/src/runtime/reconcile.rs",
        "packages/file-types/jsonc/aqc-tsconfig-json-engine/src/runtime/reconcile.rs",
        "packages/aqc-file-engine-core/src/finding.rs",
        "packages/aqc-file-engine-core/src/lib.rs",
        "packages/aqc-file-engine-core/src/merge/items.rs",
        "packages/aqc-file-engine-core/src/merge/lists.rs",
        "packages/aqc-file-engine-core/src/merge/mod.rs",
        "packages/aqc-file-engine-core/src/merge/scalar.rs",
        "packages/aqc-file-engine-core/tests/public_contract.rs",
        "packages/aqc-file-engine-core/deny.toml",
        "packages/aqc-filetree/deny.toml",
        "packages/aqc-fs-utils/deny.toml",
        "packages/aqc-git-helpers/deny.toml",
        "packages/file-types/json/aqc-json-engine-core/deny.toml",
        "packages/file-types/json/aqc-package-json-engine/deny.toml",
        "packages/file-types/jsonc/aqc-tsconfig-json-engine/deny.toml",
        "packages/file-types/text/aqc-text-file-engine/deny.toml",
        "packages/file-types/toml/aqc-cargo-toml-engine/deny.toml",
        "packages/file-types/toml/aqc-clippy-toml-engine/deny.toml",
        "packages/file-types/toml/aqc-deny-toml-engine/deny.toml",
        "packages/file-types/toml/aqc-rust-toolchain-toml-engine/deny.toml",
        "packages/file-types/toml/aqc-rustfmt-toml-engine/deny.toml",
        "packages/file-types/toml/aqc-toml-engine-core/deny.toml",
        "packages/file-types/yaml/aqc-pnpm-workspace-yaml-engine/deny.toml",
        "packages/file-types/yaml/aqc-yaml-engine-core/deny.toml",
        "packages/source/rust/aqc-rust-syntax/deny.toml",
    }
    paths = set(result.stdout.splitlines()) | set(untracked.stdout.splitlines())
    forbidden = [
        path for path in paths
        if path not in allowed_exact
    ]
    missing = sorted(allowed_exact.difference(paths))
    ok = result.returncode == 0 and untracked.returncode == 0 and not forbidden and not missing
    detail = f"out_of_scope={forbidden}; missing={missing}"
else:
    raise SystemExit(f"unknown custom check: {check}")

print(json.dumps({"check": check, "status": "pass" if ok else "fail", "message": detail}))
