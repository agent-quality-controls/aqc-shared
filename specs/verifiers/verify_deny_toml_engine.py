#!/usr/bin/env python3
from __future__ import annotations

import json
import subprocess
import sys
import tomllib
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[2]
PKG = ROOT / "packages/file-types/toml/aqc-deny-toml-engine"
TOML_CORE = ROOT / "packages/file-types/toml/aqc-toml-engine-core"
CLIPPY = ROOT / "packages/file-types/toml/aqc-clippy-toml-engine"


REQUIRED_FIELDS = [
    "graph_targets",
    "graph_exclude",
    "graph_exclude_dev",
    "graph_exclude_unpublished",
    "graph_all_features",
    "graph_no_default_features",
    "graph_features",
    "output_feature_depth",
    "advisories_version",
    "advisories_db_path",
    "advisories_db_urls",
    "advisories_yanked",
    "advisories_disable_yank_checking",
    "advisories_ignore",
    "advisories_unmaintained",
    "advisories_unsound",
    "advisories_maximum_db_staleness",
    "advisories_git_fetch_with_cli",
    "advisories_unused_ignored_advisory",
    "licenses_version",
    "licenses_include_dev",
    "licenses_include_build",
    "licenses_allow",
    "licenses_exceptions",
    "licenses_confidence_threshold",
    "licenses_clarify",
    "licenses_private_ignore",
    "licenses_private_registries",
    "licenses_private_ignore_sources",
    "licenses_unused_allowed_license",
    "licenses_unused_license_exception",
    "bans_multiple_versions",
    "bans_multiple_versions_include_dev",
    "bans_wildcards",
    "bans_allow_wildcard_paths",
    "bans_highlight",
    "bans_workspace_default_features",
    "bans_external_default_features",
    "bans_allow",
    "bans_allow_workspace",
    "bans_deny",
    "bans_features",
    "bans_skip",
    "bans_skip_tree",
    "bans_workspace_dependencies_duplicates",
    "bans_workspace_dependencies_include_path_dependencies",
    "bans_workspace_dependencies_unused",
    "bans_build_executables",
    "bans_build_interpreted",
    "bans_build_script_extensions",
    "bans_build_enable_builtin_globs",
    "bans_build_globs",
    "bans_build_include_dependencies",
    "bans_build_include_workspace",
    "bans_build_include_archives",
    "sources_unknown_registry",
    "sources_unknown_git",
    "sources_required_git_spec",
    "sources_allow_git",
    "sources_private",
    "sources_allow_registry",
    "sources_allow_org_github",
    "sources_allow_org_gitlab",
    "sources_allow_org_bitbucket",
    "sources_unused_allowed_source",
    "closed_settings",
]


REQUIRED_TYPES = [
    "DenyTomlValueError",
    "DenyLintLevel",
    "DenyAdvisoryScope",
    "DenyGraphHighlight",
    "DenyGitSpec",
    "DenyNonEmptyString",
    "DenyPackageSpec",
    "DenyDuration",
    "DenyConfidenceThreshold",
    "DenyGraphTargetSpec",
    "DenyAdvisoryIgnoreIdentity",
    "DenyAdvisoryIgnoreSpec",
    "DenyLicenseException",
    "DenyLicenseFile",
    "DenyLicenseClarification",
    "DenyPackageReasonSpec",
    "DenyBanSpec",
    "DenyFeatureBanSpec",
    "DenySkipTreeSpec",
    "DenyBuildGlobSpec",
]


REQUIRED_TESTS = [
    "missing_file",
    "writes_deterministic_baseline",
    "malformed_toml",
    "wrong_scalar_type",
    "wrong_list_member_type",
    "empty_list_member",
    "unknown_enum_value",
    "wrong_item_shape",
    "duplicate_item_identity",
    "unused_allowed_org_is_invalid_for_target_schema",
    "bans_build_is_valid_when_open",
    "closed_bans_build_removes_extra",
    "deprecated_name_repairs_to_crate",
    "feature_allow_deny_overlap_is_invalid",
    "list_order_is_ignored",
    "list_output_is_sorted",
    "missing_field_is_repaired",
    "valid_drift_is_repaired",
    "confidence_threshold_writes_float",
    "confidence_threshold_at_least_accepts_stricter_value",
    "confidence_threshold_at_least_repairs_weaker_value",
    "duration_requires_cargo_deny_shape",
    "conflicting_requirements_report_conflict",
    "uses_core_scalar_merge",
    "confidence_threshold_uses_core_ordered_scalar_merge",
    "uses_core_list_merge",
    "uses_core_item_merge",
]


FORBIDDEN_SOURCE_TEXT = [
    "shakrs",
    "shackles",
    "guardrail3",
    "g3rs",
    "CargoAdapter",
    "ToolchainAdapter",
    "Policy",
    "aqc_cargo_toml_engine",
    "aqc_clippy_toml_engine",
    "aqc_rustfmt_toml_engine",
    "aqc_rust_toolchain_toml_engine",
    "BTreeMap<String, ConfigScalar>",
    "HashMap",
    "pub scalar_settings",
    "pub list_settings",
    "sources_unused_allowed_org",
    "DenySkipSpec",
    "pub name:",
    "pub crate:",
    "pub wrappers:",
    "pub deny:",
    "pub allow:",
    "pub globs:",
]


def main() -> int:
    spec = json.loads(Path(sys.argv[1]).read_text())
    entry = spec["requirements"]["custom"][int(sys.argv[3])]
    check = entry["check"]
    if check == "engine-contract":
        print(json.dumps(engine_contract(), sort_keys=True))
    elif check == "dependency-shape":
        print(json.dumps(dependency_shape(), sort_keys=True))
    elif check == "toml-core-items":
        print(json.dumps(toml_core_items(), sort_keys=True))
    elif check == "clippy-array-reuse":
        print(json.dumps(clippy_array_reuse(), sort_keys=True))
    elif check == "cargo-tests":
        print(json.dumps(cargo_tests(), sort_keys=True))
    else:
        print(json.dumps({"status": "fail", "message": f"unknown check {check}", "check": check}))
    return 0


def pass_result(**extra: object) -> dict[str, object]:
    return {"status": "pass", **extra}


def fail_result(message: str, **extra: object) -> dict[str, object]:
    return {"status": "fail", "message": message, **extra}


def read(path: Path) -> str:
    return path.read_text(errors="replace") if path.exists() else ""


def rust_sources(root: Path) -> list[Path]:
    if not root.exists():
        return []
    return sorted(path for path in root.rglob("*.rs") if "target" not in path.parts)


def engine_contract() -> dict[str, object]:
    failures: list[str] = []
    src = PKG / "src"
    if not src.exists():
        return fail_result("missing aqc-deny-toml-engine source directory")

    lib = read(src / "lib.rs")
    engine = read(src / "engine.rs")
    model = read(src / "requirement/model.rs")
    merge = "\n".join(
        read(path)
        for path in (
            src / "requirement/merge.rs",
            src / "requirement/merge_helpers.rs",
        )
    )
    values = "\n".join(
        read(path)
        for path in sorted((src / "requirement").rglob("value*.rs"))
        + sorted((src / "requirement/value").rglob("*.rs"))
    )
    reconcile = "\n".join(read(path) for path in (src / "reconcile").glob("*.rs"))
    tests = "\n".join(
        read(PKG / f"tests/{name}")
        for name in ("parse.rs", "reconcile.rs", "merge.rs", "engine_requirement.rs")
    )
    all_rs = "\n".join(read(path) for path in rust_sources(src) + rust_sources(PKG / "tests"))

    for item in (
        "pub const ENGINE_ID: &str = \"aqc-deny-toml-engine\"",
        "pub use engine::DenyTomlEngine",
        "ScalarAssertion",
        "ListRequirements",
        "ItemRequirements",
        "ResolvedRequirement",
        "ResolvedListRequirements",
        "ResolvedItemRequirements",
        "ConflictEntry",
        "Provenance",
        "pub mod requirement",
        "pub type DenyTomlRequirements = requirement::DenyTomlRequirements",
        "pub type ResolvedDenyTomlRequirements = requirement::ResolvedDenyTomlRequirements",
    ):
        if item not in lib:
            failures.append(f"lib.rs missing {item}")

    for item in (
        "workspace_root.join(\"deny.toml\")",
        "parse_or_report(current_bytes, \"deny.toml\")",
        "merged_reconcile",
        "DenyTomlRequirements::merge",
        "impl FileEngine<ResolvedDenyTomlRequirements> for DenyTomlEngine",
        "impl Engine for DenyTomlEngine",
    ):
        if item not in engine:
            failures.append(f"engine.rs missing {item}")

    for field in REQUIRED_FIELDS:
        if f"pub {field}:" not in model:
            failures.append(f"model missing raw field {field}")
        if model.count(f"pub {field}:") < 2:
            failures.append(f"model missing resolved field {field}")

    for item in (
        "pub struct DenyTomlRequirements",
        "pub struct ResolvedDenyTomlRequirements",
        "impl EngineRequirement for DenyTomlRequirements",
    ):
        if item not in model:
            failures.append(f"model missing {item}")

    for item in (
        "resolve_maybe",
        "resolve_list",
        "resolve_items",
        "ScalarAssertion::<",
        "push_conflict",
    ):
        if item not in merge:
            failures.append(f"merge modules missing core helper {item}")

    for forbidden in ("resolve_optional_scalar", "contains_excludes", "sources_unused_allowed_org"):
        if forbidden in merge:
            failures.append(f"merge modules contain forbidden local helper {forbidden}")

    for type_name in REQUIRED_TYPES:
        if f"pub struct {type_name}" not in values and f"pub enum {type_name}" not in values:
            failures.append(f"value modules missing {type_name}")

    for item in (
        "impl ScalarValue for value::DenyLintLevel",
        "impl ScalarValue for value::DenyAdvisoryScope",
        "impl ScalarValue for value::DenyGraphHighlight",
        "impl ScalarValue for value::DenyGitSpec",
        "impl ScalarValue for value::DenyConfidenceThreshold",
        "Some(self.cmp(other))",
        "impl ScalarValue for value::DenyDuration",
        "impl ScalarValue for value::DenyNonEmptyString",
        "impl FileItemRequirement for value::DenyGraphTargetSpec",
        "impl FileItemRequirement for value::DenyAdvisoryIgnoreSpec",
        "impl FileItemRequirement for value::DenyLicenseException",
        "impl FileItemRequirement for value::DenyLicenseClarification",
        "impl FileItemRequirement for value::DenyPackageReasonSpec",
        "impl FileItemRequirement for value::DenyBanSpec",
        "impl FileItemRequirement for value::DenyFeatureBanSpec",
        "impl FileItemRequirement for value::DenySkipTreeSpec",
        "impl FileItemRequirement for value::DenyBuildGlobSpec",
    ):
        if item not in values:
            failures.append(f"value modules missing {item}")

    for item in (
        "push_mismatch",
        "attribution",
        "scalar_field_edit",
        "report_list_shape",
        "reconcile_list_field",
        "ensure_table",
        "ensure_nested",
        "table_ref",
        "reconcile_array_items",
        "reconcile_array_table_items",
    ):
        if item not in reconcile:
            failures.append(f"reconcile missing TOML core helper {item}")

    for test_name in REQUIRED_TESTS:
        if test_name not in tests:
            failures.append(f"tests missing {test_name}")

    for forbidden in FORBIDDEN_SOURCE_TEXT:
        if forbidden in all_rs:
            failures.append(f"forbidden source text {forbidden}")

    if "Finding::Mismatch" in reconcile:
        failures.append("reconcile must use TOML-core finding helpers instead of direct Finding::Mismatch")

    if failures:
        return fail_result("deny TOML engine contract failed", failures=failures)
    return pass_result()


def dependency_shape() -> dict[str, object]:
    manifest = PKG / "Cargo.toml"
    if not manifest.exists():
        return fail_result("missing engine Cargo.toml")
    data = tomllib.loads(manifest.read_text())
    failures: list[str] = []
    package = data.get("package", {})
    if package.get("name") != "aqc-deny-toml-engine":
        failures.append("package.name must be aqc-deny-toml-engine")
    if package.get("publish") is False:
        failures.append("package must be publishable")
    if not package.get("readme"):
        failures.append("package must declare readme")
    if not package.get("description"):
        failures.append("package must declare description")
    if "workspace" not in data:
        failures.append("crate must be an independent workspace")
    deps: dict[str, Any] = {}
    for section in ("dependencies", "dev-dependencies", "build-dependencies"):
        value = data.get(section, {})
        if isinstance(value, dict):
            deps.update(value)
    for dep, value in deps.items():
        if dep.startswith("shakrs") or dep == "shackles":
            failures.append(f"forbidden Shackles dependency {dep}")
        if dep in {
            "aqc-cargo-toml-engine",
            "aqc-clippy-toml-engine",
            "aqc-rustfmt-toml-engine",
            "aqc-rust-toolchain-toml-engine",
        }:
            failures.append(f"forbidden concrete engine dependency {dep}")
        if isinstance(value, dict):
            for key in ("path", "git", "workspace"):
                if key in value:
                    failures.append(f"dependency {dep} uses forbidden {key}")
    if failures:
        return fail_result("dependency shape failed", failures=failures)
    return pass_result()


def toml_core_items() -> dict[str, object]:
    failures: list[str] = []
    lib = read(TOML_CORE / "src/lib.rs")
    item_sources = {
        "mod": read(TOML_CORE / "src/items/mod.rs"),
        "types": read(TOML_CORE / "src/items/types.rs"),
        "support": read(TOML_CORE / "src/items/support.rs"),
        "array": read(TOML_CORE / "src/items/array.rs"),
        "array_table": read(TOML_CORE / "src/items/array_table.rs"),
    }
    items = "\n".join(item_sources.values())
    if not all(item_sources.values()):
        return fail_result("missing TOML core items module")

    for item in (
        "mod items",
        "TomlArrayItem",
        "TomlArrayTableItem",
        "TomlItemError",
        "TomlItemField",
        "reconcile_array_items",
        "reconcile_array_table_items",
    ):
        if item not in lib:
            failures.append(f"toml core lib.rs missing {item}")

    for item in (
        "pub struct TomlItemError",
        "pub fn message(&self) -> &str",
        "fn read_value(value: &Value) -> Result<Self, TomlItemError>",
        "fn read_table(table: &dyn TableLike) -> Result<Self, TomlItemError>",
        "ResolvedItemRequirements",
        "FileItemRequirement",
        "push_mismatch",
        "ensure_array_of_tables",
        "ensure_table_at",
    ):
        if item not in items:
            failures.append(f"items.rs missing {item}")

    for item in ("Result<Self, String>", "pub table_path", "pub field_key", "pub display_key"):
        if item in items:
            failures.append(f"items module contains forbidden public API shape {item}")

    for forbidden in (
        "DenyToml",
        "aqc_deny_toml_engine",
        "CargoToml",
        "ClippyToml",
        "RustfmtToml",
        "RustToolchain",
    ):
        if forbidden in items:
            failures.append(f"items.rs contains concrete engine coupling {forbidden}")

    if failures:
        return fail_result("TOML core items contract failed", failures=failures)
    return pass_result()


def clippy_array_reuse() -> dict[str, object]:
    failures: list[str] = []
    path = CLIPPY / "src/reconcile/disallowed.rs"
    source = read(path)
    if not source:
        return fail_result("missing Clippy disallowed reconciliation")

    for item in (
        "TomlArrayItem",
        "TomlItemField",
        "reconcile_array_items",
        "impl TomlArrayItem for DisallowedEntry",
    ):
        if item not in source:
            failures.append(f"Clippy disallowed reconciliation missing {item}")

    for item in (
        "fn apply_required",
        "fn apply_forbidden(",
        "fn prune_extras(",
        "fn push_malformed_array_finding",
        "collect_current_paths",
        "positions_with_path",
        "position_with_path",
    ):
        if item in source:
            failures.append(f"Clippy disallowed reconciliation still contains local array mechanic {item}")

    if "apply_forbidden_path_globs" not in source:
        failures.append("Clippy forbidden path-glob reconciliation must stay local")

    if "aqc_deny_toml_engine" in source or "DenyToml" in source:
        failures.append("Clippy disallowed reconciliation must not couple to deny engine")

    manifest = CLIPPY / "Cargo.toml"
    if not manifest.exists():
        failures.append("missing Clippy Cargo.toml")
    else:
        data = tomllib.loads(manifest.read_text())
        deps: dict[str, Any] = {}
        for section in ("dependencies", "dev-dependencies", "build-dependencies"):
            value = data.get(section, {})
            if isinstance(value, dict):
                deps.update(value)
        if "aqc-toml-engine-core" not in deps:
            failures.append("Clippy engine must depend on aqc-toml-engine-core")
        for dep in (
            "aqc-deny-toml-engine",
            "aqc-cargo-toml-engine",
            "aqc-rustfmt-toml-engine",
            "aqc-rust-toolchain-toml-engine",
        ):
            if dep in deps:
                failures.append(f"Clippy engine has forbidden concrete engine dependency {dep}")

    if failures:
        return fail_result("Clippy array reuse contract failed", failures=failures)
    return pass_result()


def cargo_tests() -> dict[str, object]:
    manifests = [
        TOML_CORE / "Cargo.toml",
        CLIPPY / "Cargo.toml",
        PKG / "Cargo.toml",
    ]
    for manifest in manifests:
        if not manifest.exists():
            return fail_result(f"missing Cargo.toml: {manifest.relative_to(ROOT)}")
    failures = []
    for manifest in manifests:
        proc = subprocess.run(
            [
                "cargo",
                "test",
                "--manifest-path",
                str(manifest),
                "--all-targets",
            ],
            cwd=ROOT,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            check=False,
        )
        if proc.returncode != 0:
            failures.append(
                {
                    "manifest": str(manifest.relative_to(ROOT)),
                    "exit_code": proc.returncode,
                    "stdout": proc.stdout[-3000:],
                    "stderr": proc.stderr[-3000:],
                }
            )
    if failures:
        return fail_result("cargo test failed", failures=failures)
    return pass_result()


if __name__ == "__main__":
    raise SystemExit(main())
