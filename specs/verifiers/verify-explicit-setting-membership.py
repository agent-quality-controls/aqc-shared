#!/usr/bin/env python3
import json
import hashlib
import os
import subprocess
import sys
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path


SPEC = Path(sys.argv[1]).resolve()
ENTRY = json.loads(SPEC.read_text())["requirements"]["custom"][int(sys.argv[3])]
ROOT = SPEC.parent.parent
TARGET_ROOT = Path(
    os.environ.get("AQC_GATE_CACHE_DIR", ROOT / ".cargo-target" / "gate")
) / "targets"

PACKAGES = {
    "core": ROOT / "packages/aqc-file-engine-core",
    "json_core": ROOT / "packages/file-types/json/aqc-json-engine-core",
    "json": ROOT / "packages/file-types/json/aqc-json-file-engine",
    "toml": ROOT / "packages/file-types/toml/aqc-toml-engine-core",
    "cargo": ROOT / "packages/file-types/toml/aqc-cargo-toml-engine",
    "clippy": ROOT / "packages/file-types/toml/aqc-clippy-toml-engine",
    "deny": ROOT / "packages/file-types/toml/aqc-deny-toml-engine",
    "toolchain": ROOT / "packages/file-types/toml/aqc-rust-toolchain-toml-engine",
    "rustfmt": ROOT / "packages/file-types/toml/aqc-rustfmt-toml-engine",
    "yaml": ROOT / "packages/file-types/yaml/aqc-yaml-engine-core",
    "pnpm": ROOT / "packages/file-types/yaml/aqc-pnpm-workspace-yaml-engine",
    "architecture": ROOT / "tools/aqc-requirement-architecture",
}

def emit(errors: list[str], executed: list[str]) -> None:
    evidence = {
        "check": ENTRY["check"],
        "status": "fail" if errors else "pass",
        "executed": executed,
    }
    if errors:
        evidence["message"] = "; ".join(errors)
    print(json.dumps(evidence))


def files_text(base: Path, patterns: tuple[str, ...]) -> str:
    if not base.exists():
        return ""
    paths: set[Path] = set()
    for pattern in patterns:
        paths.update(path for path in base.glob(pattern) if path.is_file())
    return "\n".join(path.read_text(errors="replace") for path in sorted(paths))


def source_text(package: Path) -> str:
    return files_text(package, ("src/**/*.rs",))


def test_text(package: Path) -> str:
    return files_text(package, ("tests/**/*.rs", "src/**/*.rs"))


def require_terms(errors: list[str], label: str, text: str, groups: list[tuple[str, ...]]) -> None:
    lowered = text.lower()
    if not lowered:
        errors.append(f"{label}: source or tests are missing")
        return
    for alternatives in groups:
        if not any(term.lower() in lowered for term in alternatives):
            errors.append(f"{label}: missing evidence for {'/'.join(alternatives)}")


def require_declared_cases(errors: list[str], label: str, text: str) -> None:
    for test in ENTRY.get("requiredTests", []):
        if f"fn {test}(" not in text:
            errors.append(f"{label}: declared test {test} is not implemented")


def cargo_environment(manifest: Path) -> dict[str, str]:
    identity = hashlib.sha256(str(manifest.relative_to(ROOT)).encode()).hexdigest()[:16]
    gate_root = TARGET_ROOT.parent
    scope = os.environ.get("AQC_GATE_CONFIG_SCOPE", "working-tree")
    cargo_home = gate_root / "cargo-homes" / scope / identity
    cargo_cache = gate_root / "cargo-cache"
    cargo_home.mkdir(parents=True, exist_ok=True)
    cargo_cache.mkdir(parents=True, exist_ok=True)
    for directory in ("registry", "git"):
        shared = cargo_cache / directory
        shared.mkdir(exist_ok=True)
        link = cargo_home / directory
        if not link.exists() and not link.is_symlink():
            try:
                link.symlink_to(shared, target_is_directory=True)
            except FileExistsError:
                pass
    subprocess.run(
        [
            "python3",
            str(ROOT / "scripts/local_cargo_source.py"),
            "--root",
            str(ROOT),
            "--config",
            str(cargo_home / "config.toml"),
            "--manifest",
            str(manifest),
        ],
        check=True,
        timeout=15,
    )
    return {
        **os.environ,
        "CARGO_HOME": str(cargo_home),
        "CARGO_TARGET_DIR": str(TARGET_ROOT / identity),
    }


def cargo_command(name: str, extra: list[str]) -> subprocess.CompletedProcess[str]:
    package = PACKAGES[name]
    manifest = package / "Cargo.toml"
    command = [
        "cargo",
        "test",
        "--manifest-path",
        str(manifest),
        "--locked",
        "--quiet",
    ]
    command.extend(extra)
    return subprocess.run(
        command,
        cwd=ROOT,
        env=cargo_environment(manifest),
        text=True,
        capture_output=True,
        timeout=45,
        check=False,
    )


def cargo_test(name: str) -> str | None:
    package = PACKAGES[name]
    if not (package / "Cargo.toml").is_file():
        return f"{package.relative_to(ROOT)}: Cargo.toml is missing"
    try:
        result = cargo_command(name, [])
    except subprocess.TimeoutExpired:
        return f"{package.relative_to(ROOT)}: cargo test timed out"
    if result.returncode:
        tail = "\n".join((result.stdout + result.stderr).splitlines()[-8:])
        return f"{package.relative_to(ROOT)}: cargo test failed ({result.returncode}): {tail}"
    return None


def cargo_test_names(name: str) -> tuple[set[str], str | None]:
    try:
        result = cargo_command(name, ["--", "--list"])
    except subprocess.TimeoutExpired:
        return set(), f"{PACKAGES[name].relative_to(ROOT)}: cargo test --list timed out"
    if result.returncode:
        tail = "\n".join((result.stdout + result.stderr).splitlines()[-8:])
        return set(), f"{PACKAGES[name].relative_to(ROOT)}: cargo test --list failed ({result.returncode}): {tail}"
    names = {
        line.rsplit(": ", 1)[0]
        for line in result.stdout.splitlines()
        if line.endswith(": test")
    }
    return names, None


def run_tests(errors: list[str], executed: list[str], names: list[str]) -> None:
    with ThreadPoolExecutor(max_workers=len(names)) as executor:
        results = list(executor.map(cargo_test, names))
    for name, failure in zip(names, results):
        executed.append(f"cargo test {PACKAGES[name].relative_to(ROOT)}")
        if failure:
            errors.append(failure)
    if errors:
        return
    with ThreadPoolExecutor(max_workers=len(names)) as executor:
        listed = list(executor.map(cargo_test_names, names))
    for _, failure in listed:
        if failure:
            errors.append(failure)
    inventories = {name: tests for name, (tests, _) in zip(names, listed)}
    required = ENTRY.get("requiredTests", [])
    owners = ENTRY.get("requiredTestOwners", [])
    if len(required) != len(owners):
        errors.append("requiredTests and requiredTestOwners must have equal length")
        return
    for test, owner in zip(required, owners):
        if owner not in inventories:
            errors.append(f"declared test {test} names unchecked owner {owner}")
        elif test not in inventories[owner]:
            errors.append(f"declared test {test} is not executable in {owner}")


def core_semantics(errors: list[str], executed: list[str]) -> None:
    source = source_text(PACKAGES["core"])
    tests = test_text(PACKAGES["core"])
    require_terms(
        errors,
        "core source",
        source,
        [
            ("pub struct ItemPresenceDifference",),
            ("pub fn item_presence_difference",),
            ("pub fn map",),
            ("RequiredItemResolution",),
            ("ForbiddenItemResolution",),
            ("FileKeyRequirement",),
            ("resolve_key_membership",),
        ],
    )
    require_declared_cases(errors, "core", source + tests)
    require_terms(
        errors,
        "core tests",
        tests,
        [
            ("missing",),
            ("forbidden",),
            ("unexpected", "extra"),
            ("exact_empty", "exact empty"),
            ("duplicate",),
            ("compatible", "agreeing"),
            ("incompatible", "conflict"),
            ("required_outside", "required outside"),
            ("forbidden_inside", "forbidden inside"),
            ("provenance", "attribution", "contributors"),
            ("map_preserves", "map preserves", "mapped"),
            ("derived_key_constraints", "derived key constraints"),
            ("derived_key_constraints_are_checked_against_explicit_membership",),
            ("one_rejection_per_identity", "one rejection per identity"),
        ],
    )
    if not errors:
        run_tests(errors, executed, ["core"])


def format_reconciliation(errors: list[str], executed: list[str]) -> None:
    sources = {
        "JSON": source_text(PACKAGES["json"]),
        "Cargo": source_text(PACKAGES["cargo"]),
        "TOML": source_text(PACKAGES["toml"]),
        "YAML": source_text(PACKAGES["yaml"]),
    }
    for label, source in sources.items():
        require_terms(errors, f"{label} source", source, [("item_presence_difference",)])
    require_terms(
        errors,
        "format tests",
        "\n".join(
            test_text(PACKAGES[name])
            for name in ("json", "cargo", "clippy", "toml", "yaml")
        ),
        [
            ("object", "json"),
            ("lint", "cargo"),
            ("table_key", "table key"),
            ("effective", "merge key", "merge_key"),
            ("inherited", "anchor"),
            ("unwritable", "cannot construct", "not construct"),
            ("parent_removal_precedes_child_reconciliation",),
        ],
    )
    require_declared_cases(
        errors,
        "format reconciliation",
        "\n".join(sources.values())
        + "\n"
        + "\n".join(
            test_text(PACKAGES[name])
            for name in ("json", "cargo", "clippy", "toml", "yaml")
        ),
    )
    if not errors:
        run_tests(errors, executed, ["json", "cargo", "clippy", "toml", "yaml"])


def migrated_engines(errors: list[str], executed: list[str]) -> None:
    surfaces = {
        "rustfmt": "setting_keys",
        "toolchain": "toolchain_keys",
        "pnpm": "root_keys",
        "deny": "table_keys",
    }
    for name, field in surfaces.items():
        source = source_text(PACKAGES[name])
        require_terms(
            errors,
            f"{name} source",
            source,
            [(field,), ("ItemRequirements",), ("ResolvedItemRequirements",)],
        )
        if "exact_settings" in source or "closed_settings" in source:
            errors.append(f"{name} source retains a semantic closure marker")
        require_terms(
            errors,
            f"{name} tests",
            test_text(PACKAGES[name]),
            [
                ("missing",),
                ("forbidden",),
                ("extra", "unexpected", "unknown"),
                ("absent",),
                ("conflict",),
                ("init", "initialize"),
                ("second", "idempotent"),
                ("provenance", "attribution", "contributors"),
                ("cannot_exclude_a_constructive", "value and membership requirements must conflict"),
            ],
        )
    require_terms(
        errors,
        "Rustfmt regressions",
        test_text(PACKAGES["rustfmt"]),
        [("nightly",), ("ignore",), ("absent",)],
    )
    require_declared_cases(
        errors,
        "migrated engines",
        "\n".join(
            source_text(PACKAGES[name]) + test_text(PACKAGES[name])
            for name in ("deny", "rustfmt", "toolchain", "pnpm")
        ),
    )
    require_terms(
        errors,
        "toolchain regressions",
        test_text(PACKAGES["toolchain"]),
        [
            ("path",),
            ("targets",),
            ("empty", "no_targets", "no targets"),
            ("exact_empty_toolchain_membership_fails_merge",),
        ],
    )
    require_terms(
        errors,
        "pnpm regressions",
        test_text(PACKAGES["pnpm"]),
        [
            ("merge", "effective"),
            ("direct",),
            ("inherited", "anchor"),
            ("glob",),
            ("absence", "absent"),
            ("rejected_root_key_has_one_membership_finding",),
            ("inherited_rejected_root_key_has_one_membership_finding",),
            ("invalid_merge_source_stops_child_reconciliation",),
            ("rejected_anchor_owner_is_not_removed_or_followed_by_a_child_finding",),
        ],
    )
    require_terms(
        errors,
        "Deny table coverage",
        test_text(PACKAGES["deny"]),
        [
            ("graph",),
            ("output",),
            ("advisories",),
            ("licenses",),
            ("private",),
            ("workspace_dependencies", "workspace-dependencies"),
            ("build",),
            ("sources",),
            ("allow_org", "allow-org"),
        ],
    )
    if not errors:
        run_tests(errors, executed, ["deny", "rustfmt", "toolchain", "pnpm"])


def architecture_checker(errors: list[str], executed: list[str]) -> None:
    package = PACKAGES["architecture"]
    source = source_text(package)
    fixtures_and_tests = files_text(package, ("tests/**/*", "fixtures/**/*", "src/**/*.rs"))
    require_terms(
        errors,
        "architecture checker source",
        source,
        [
            ("EngineRequirement",),
            ("AdapterRequirement",),
            ("ItemRequirements",),
            ("AdapterMembershipConstruction",),
            ("NonCanonicalRequirementRoot",),
            ("resolve_scoped_path",),
            ("visit_expr_closure",),
            ("visit_macro",),
            ("visit_expr_method_call",),
            ("visit_expr_reference",),
            ("visit_pat_struct",),
            ("cargo_metadata",),
            ("syn",),
            ("inventory", "RequirementRoot", "roots"),
        ],
    )
    require_declared_cases(errors, "architecture checker", source + fixtures_and_tests)
    require_terms(
        errors,
        "architecture adversarial tests",
        fixtures_and_tests,
        [
            ("exact_settings",),
            ("closed_settings",),
            ("alias",),
            ("wrapped",),
            ("tuplerequirementroot", "aliasedrequirementroot"),
            ("hiddenadapter", "requirementcontract"),
            ("inferred_required",),
            ("inferred_membership",),
            ("inferred_by_extend",),
            ("inferred_by_mutable_reference",),
            ("inferred_by_destructuring",),
            ("default_membership_replacement",),
            ("noncanonical_membership_field", "NoncanonicalMembershipField"),
            ("local_macro_alias",),
            ("renamed_local_membership_mutation",),
            ("hidden_default_construction",),
            ("rewrite_membership_parameter",),
            ("PrivateClosureField",),
            ("ReimplementedCoreVocabulary",),
            ("UninspectableRequirementMacro",),
            ("imported_membership_alias",),
            ("local_helper_produced_engine_membership_is_rejected",),
            ("cross_crate_helper_produced_engine_membership_is_rejected",),
            ("closure_membership_parameter_is_rejected",),
            ("inferred_closure_membership_is_rejected",),
            ("parent_module_trait_alias_is_inventoried",),
            ("tuple_whole_engine_helper",),
            ("tuple_struct_whole_engine_helper",),
            ("reassigned_whole_engine_helper",),
            ("type_annotated_membership_local_is_tracked_and_rejected",),
            ("unrelated_keys_field_is_accepted",),
            ("canonical_origin_rejects_terminal_name_counterfeits",),
            ("canonical_origin_accepts_public_reexport_chain",),
            ("canonical_origin_rejects_nested_counterfeit_from_mixed_facade",),
            ("direct_transfer_typed_local_and_map_are_accepted",),
            ("unrelated_required_field_mutation",),
            ("unrelated_nested_required_field_mutation",),
            ("unrelated_external_macro",),
            ("The adversarial case", "must produce its own violation"),
            ("policy",),
            (".map", "map("),
            ("inventory",),
            ("../shackles", "repository root", "repository_root"),
        ],
    )
    if not errors:
        run_tests(errors, executed, ["architecture"])
    if errors:
        return
    repository_roots = [(ROOT / relative).resolve() for relative in ENTRY["repositoryRoots"]]
    try:
        environment = cargo_environment(package / "Cargo.toml")
        result = subprocess.run(
            [
                "cargo",
                "run",
                "--quiet",
                "--locked",
                "--manifest-path",
                str(package / "Cargo.toml"),
                "--",
                str(PACKAGES["core"] / "Cargo.toml"),
                *(str(root) for root in repository_roots),
            ],
            cwd=ROOT,
            env=environment,
            text=True,
            capture_output=True,
            timeout=120,
            check=False,
        )
    except subprocess.TimeoutExpired:
        errors.append("architecture checker repository scan timed out")
        return
    executed.append(
        "aqc-requirement-architecture "
        + str((PACKAGES["core"] / "Cargo.toml").relative_to(ROOT))
        + " "
        + " ".join(str(root.relative_to(ROOT.parent)) for root in repository_roots)
    )
    if result.returncode:
        tail = "\n".join((result.stdout + result.stderr).splitlines()[-12:])
        errors.append(f"architecture checker repository scan failed ({result.returncode}): {tail}")
        return
    try:
        report = json.loads(result.stdout)
    except json.JSONDecodeError as error:
        errors.append(f"architecture checker repository scan returned invalid JSON: {error}")
        return
    observed_roots = {
        f'{root["crate_name"]}:{root["name"]}:{root["kind"]}'
        for root in report.get("roots", [])
    }
    expected_roots = set(ENTRY.get("requiredProductionRoots", []))
    if observed_roots != expected_roots:
        errors.append(
            "architecture checker production inventory differs: "
            f"expected {sorted(expected_roots)}, observed {sorted(observed_roots)}"
        )


CHECKS = {
    "core-item-presence-semantics": core_semantics,
    "format-membership-reconciliation": format_reconciliation,
    "migrated-engine-behavior": migrated_engines,
    "architecture-checker-semantics": architecture_checker,
}


errors: list[str] = []
executed: list[str] = []
handler = CHECKS.get(ENTRY.get("check"))
if handler is None:
    errors.append(f"unsupported check: {ENTRY.get('check')}")
else:
    handler(errors, executed)
emit(errors, executed)
